#![cfg(feature = "std")]
//! Ogg Vorbis files, over the `lewton` crate. Besides the samples it keeps what
//! a loop importer needs: the file's comments, and where in the sound it is.
//!
//! [`seek_page`](VorbisSource::seek_page) lands on a page, not a sample: the
//! first page whose end is at or past the frame asked for. The packet after a
//! seek decodes to nothing, as each overlaps the one before, and
//! [`position`](VorbisSource::position) isn't known until that page ends. So an
//! exact boundary seeks to a frame some way before it, reads on until the
//! position is known and is still before the boundary, and throws away the
//! frames up to it.

use std::io::Cursor;

use lewton::VorbisError;
use lewton::audio::AudioReadError;
use lewton::inside_ogg::OggStreamReader;

use crate::error::Error;
use crate::guard::guarded;
use crate::pcm::{Limits, PcmFormat};
use crate::source::{Source, room};

const FORMAT: &str = "Ogg Vorbis";
/// How far from a file's end its last page can start: the most an Ogg page is.
const MAX_PAGE_BYTES: usize = 65_307;
/// Where a page's header holds its flags, granule position and serial number.
const FLAGS_AT: usize = 5;
const GRANULE_AT: usize = 6;
const SERIAL_AT: usize = 14;
/// The flag of a stream's last page.
const LAST_PAGE: u8 = 4;

/// The granule position and serial number of the page that ends `bytes`, if it
/// is a stream's last, as its flag says.
fn last_page(bytes: &[u8]) -> Option<(u64, u32)> {
    let tail = &bytes[bytes.len().saturating_sub(MAX_PAGE_BYTES)..];
    let start = tail.windows(4).rposition(|magic| magic == b"OggS")?;
    let page = &tail[start..];
    if page.get(FLAGS_AT)? & LAST_PAGE == 0 {
        return None;
    }
    let granule = page.get(GRANULE_AT..GRANULE_AT + 8)?;
    let serial = page.get(SERIAL_AT..SERIAL_AT + 4)?;
    Some((
        u64::from_le_bytes(granule.try_into().ok()?),
        u32::from_le_bytes(serial.try_into().ok()?),
    ))
}

/// An Ogg Vorbis file's samples, decoded from its bytes as they are asked for.
pub struct VorbisSource<'a> {
    reader: OggStreamReader<Cursor<&'a [u8]>>,
    format: PcmFormat,
    max_frames: u64,
    /// How many frames it has decoded, to check against `max_frames`.
    decoded: u64,
    /// What the last page says is the stream's end, if there is one stream.
    end: Option<u64>,
    /// Where the end of the last packet is, in frames from the start of the
    /// sound. A seek forgets it until a page's end says.
    at: Option<u64>,
    /// The last packet's samples, and how many of them have gone out.
    packet: Vec<i16>,
    sent: usize,
    ended: bool,
    /// The library failed: nothing more comes of this reader.
    broken: bool,
}

fn malformed(error: &VorbisError) -> Error {
    Error::Malformed {
        format: FORMAT,
        reason: format!("{error:?}"),
    }
}

impl<'a> VorbisSource<'a> {
    /// A source of `bytes`, whose headers say what it holds, which must be
    /// within `limits`. It decodes no audio yet.
    pub fn new(bytes: &'a [u8], limits: &Limits) -> Result<Self, Error> {
        guarded(FORMAT, || {
            let reader = OggStreamReader::new(Cursor::new(bytes)).map_err(|e| malformed(&e))?;
            let format = PcmFormat::new(
                reader.ident_hdr.audio_sample_rate,
                u32::from(reader.ident_hdr.audio_channels),
                limits,
            )?;
            let serial = reader.stream_serial();
            let end = last_page(bytes)
                .filter(|(_, last)| *last == serial)
                .map(|(granule, _)| granule);
            Ok(Self {
                reader,
                format,
                max_frames: limits.max_frames,
                decoded: 0,
                end,
                at: Some(0),
                packet: Vec::new(),
                sent: 0,
                ended: false,
                broken: false,
            })
        })
    }

    /// The file's comments, as it has them, each a name and its value.
    pub fn comments(&self) -> &[(String, String)] {
        &self.reader.comment_hdr.comment_list
    }

    /// The first comment called `name`, whatever its case, as Vorbis says names
    /// are compared.
    pub fn comment(&self, name: &str) -> Option<&str> {
        self.comments()
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The name of what encoded the file.
    pub fn vendor(&self) -> &str {
        &self.reader.comment_hdr.vendor
    }

    /// What the file's last page says is its end, in frames from the start of
    /// the sound, if it has such a page. Nothing has checked it yet.
    pub fn end_granule(&self) -> Option<u64> {
        self.end
    }

    /// Where the next frame read is, in frames from the start of the sound. After
    /// a seek it isn't known until the end of a page has been decoded.
    pub fn position(&self) -> Option<u64> {
        let channels = usize::from(self.format.channels.count());
        let waiting = ((self.packet.len() - self.sent) / channels) as u64;
        self.at.map(|at| at.saturating_sub(waiting))
    }

    /// Moves to the first page that ends at or past `frame`, or to the file's end
    /// if none does, dropping what was decoded of the last packet. The module's
    /// documentation says how to get from there to a sample. What it has decoded
    /// still counts against the limit.
    pub fn seek_page(&mut self, frame: u64) -> Result<(), Error> {
        self.check_broken()?;
        let reader = &mut self.reader;
        let sought = guarded(FORMAT, || {
            reader.seek_absgp_pg(frame).map_err(|e| malformed(&e))
        });
        self.broken = sought.is_err();
        sought?;
        self.packet.clear();
        self.sent = 0;
        self.ended = false;
        self.at = None;
        Ok(())
    }

    fn check_broken(&self) -> Result<(), Error> {
        if self.broken {
            return Err(Error::Malformed {
                format: FORMAT,
                reason: "it failed earlier, and the decoder can't go on from there".to_owned(),
            });
        }
        Ok(())
    }

    /// Decodes the next packet, or says the file has ended.
    fn next_packet(&mut self) -> Result<(), Error> {
        let reader = &mut self.reader;
        let packet = guarded(FORMAT, || match reader.read_dec_packet_itl() {
            // A seek near the start lands among the headers; the sound follows.
            Err(VorbisError::BadAudio(AudioReadError::AudioIsHeader)) => Ok(Some(Vec::new())),
            other => other.map_err(|e| malformed(&e)),
        });
        self.broken = packet.is_err();
        let Some(mut samples) = packet? else {
            self.ended = true;
            return Ok(());
        };
        let channels = u64::from(self.format.channels.count());
        let count = samples.len() as u64;
        if !count.is_multiple_of(channels) {
            // A stream that changed its channels partway.
            self.broken = true;
            return Err(Error::PartialFrame {
                samples: count,
                channels: self.format.channels.count(),
            });
        }
        let frames = count / channels;
        self.decoded += frames;
        // Only a page's end is exact: the library's count between them isn't.
        let at = self
            .at
            .map_or(self.reader.get_last_absgp(), |at| Some(at + frames));
        // The last packet decodes whole, and the stream's end cuts it. The library
        // does that only once a page has ended before it, which in a file of one
        // page of sound none has.
        let extra = at
            .zip(self.end)
            .map_or(0, |(at, end)| at.saturating_sub(end))
            .min(frames);
        samples.truncate(usize::try_from((frames - extra) * channels).unwrap_or(0));
        self.at = at.map(|at| at - extra);
        if self.decoded > self.max_frames {
            self.broken = true;
            return Err(Error::TooLong {
                frames: self.decoded,
                limit: self.max_frames,
            });
        }
        self.packet = samples;
        self.sent = 0;
        Ok(())
    }
}

impl Source for VorbisSource<'_> {
    fn format(&self) -> PcmFormat {
        self.format
    }

    /// Not known without decoding: the last page says, and a file can lie.
    fn total_frames(&self) -> Option<u64> {
        None
    }

    fn read(&mut self, out: &mut [i16]) -> Result<usize, Error> {
        let room = room(self.format, out)?;
        self.check_broken()?;
        let channels = usize::from(self.format.channels.count());
        let wanted = room * channels;
        let mut written = 0;
        while written < wanted {
            if self.sent == self.packet.len() {
                if self.ended {
                    break;
                }
                self.next_packet()?;
                continue;
            }
            let count = (wanted - written).min(self.packet.len() - self.sent);
            out[written..written + count]
                .copy_from_slice(&self.packet[self.sent..self.sent + count]);
            self.sent += count;
            written += count;
        }
        Ok(written / channels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An Ogg page header with these fields and no packets.
    fn page(flags: u8, granule: u64, serial: u32) -> Vec<u8> {
        let mut out = b"OggS\0".to_vec();
        out.push(flags);
        out.extend_from_slice(&granule.to_le_bytes());
        out.extend_from_slice(&serial.to_le_bytes());
        out.extend_from_slice(&[0; 9]);
        out
    }

    #[test]
    fn the_last_page_is_read_from_its_own_fields() {
        let mut bytes = page(0, 1, 7);
        bytes.extend_from_slice(&page(LAST_PAGE, 22_050, 9));
        assert_eq!(last_page(&bytes), Some((22_050, 9)));
    }

    #[test]
    fn a_last_page_without_its_flag_or_cut_short_says_nothing() {
        assert_eq!(last_page(&page(0, 22_050, 9)), None);
        assert_eq!(last_page(b""), None);
        assert_eq!(last_page(b"no page here"), None);
        let whole = page(LAST_PAGE, 22_050, 9);
        for cut in 4..18 {
            assert_eq!(last_page(&whole[..cut]), None, "{cut}");
        }
    }
}
