// The C side of the Playdate backend (ADR-0062). The SDK's build support
// (setup.c, link_map.ld) does the platform part; this file is the SDK's entry
// point and one small wrapper per SDK call `src/device/` uses. It is compiled
// against the installed SDK's pd_api.h, so the C compiler checks every call
// against the real API. The SDK's headers are never copied into the repository.
#include <stdint.h>

#include "pd_api.h"

static PlaydateAPI* pd;

// The font the card and the errors are drawn in, if the system has it.
static LCDFont* font;

// The game's crate (`shuyi_playdate::main!`).
extern void shuyi_playdate_init(void);
extern int shuyi_playdate_update(void);
extern void shuyi_playdate_event(int event);
extern void shuyi_playdate_menu_chosen(int slot);

// --- memory, console, errors ------------------------------------------------

void* pd_realloc(void* ptr, size_t size) { return pd->system->realloc(ptr, size); }
void pd_log(const char* text) { pd->system->logToConsole("%s", text); }

#ifdef TARGET_SIMULATOR
// On the Simulator `error` shows the message and comes back, on the thread its
// window runs on. So a panic leaves the game's call for good and no call goes
// in again, and the window stays alive to show it. Nothing the panic jumps over
// runs again, so no destructor is missed.
static void* stop_here[5];
static int stopped;
#define CALL_GAME(call, otherwise) \
	do { \
		if (stopped || __builtin_setjmp(stop_here)) { \
			return otherwise; \
		} \
		call; \
	} while (0)
#else
#define CALL_GAME(call, otherwise) call
#endif

void pd_error(const char* text)
{
	pd->system->error("%s", text);
#ifdef TARGET_SIMULATOR
	stopped = 1;
	__builtin_longjmp(stop_here, 1);
#endif
}

// --- time -------------------------------------------------------------------

unsigned int pd_millis(void) { return pd->system->getCurrentTimeMilliseconds(); }
float pd_elapsed(void) { return pd->system->getElapsedTime(); }

// --- buttons ----------------------------------------------------------------

// `src/input.rs` keeps a copy of these bits; the compiler checks it here.
_Static_assert(kButtonLeft == 1 && kButtonRight == 2 && kButtonUp == 4 && kButtonDown == 8
		&& kButtonB == 16 && kButtonA == 32,
	"the button bits in src/input.rs no longer match pd_api_sys.h");

// The buttons held now and the ones pressed since the last call.
void pd_buttons(unsigned int* current, unsigned int* pushed)
{
	PDButtons held, down, up;
	pd->system->getButtonState(&held, &down, &up);
	*current = (unsigned int)held;
	*pushed = (unsigned int)down;
}

// --- the crank --------------------------------------------------------------

// Degrees turned since the last call, clockwise positive.
float pd_crank_change(void) { return pd->system->getCrankChange(); }
int pd_crank_docked(void) { return pd->system->isCrankDocked(); }

// --- the system menu --------------------------------------------------------

// The SDK calls this when the player chooses one of the game's items;
// `userdata` is the item's slot, handed on to the Rust side.
static void menu_chosen(void* userdata)
{
	CALL_GAME(shuyi_playdate_menu_chosen((int)(intptr_t)userdata), );
}

int pd_add_menu_item(const char* title, int slot)
{
	return pd->system->addMenuItem(title, menu_chosen, (void*)(intptr_t)slot) != NULL;
}

// --- the display ------------------------------------------------------------

// `src/display.rs` writes the frame buffer as `shuyi_ui::frame::onebit` lays
// it out.
_Static_assert(LCD_ROWSIZE == 52 && LCD_ROWS == 240,
	"the frame buffer is no longer 52 bytes by 240 rows (shuyi_ui::frame::onebit)");

void pd_set_refresh_rate(float rate) { pd->display->setRefreshRate(rate); }
unsigned char* pd_frame(void) { return pd->graphics->getFrame(); }
void pd_mark_updated_rows(int start, int end) { pd->graphics->markUpdatedRows(start, end); }

// The whole screen `white` or black, and `text` centred on it in the other,
// wrapped by words, in the system's font.
void pd_draw_text(const char* text, size_t len, int white)
{
	pd->graphics->clear(white ? kColorWhite : kColorBlack);
	pd->graphics->setFont(font);
	pd->graphics->setDrawMode(white ? kDrawModeFillBlack : kDrawModeFillWhite);
	pd->graphics->drawTextInRect(text, len, kUTF8Encoding, 8, 8, LCD_COLUMNS - 16, LCD_ROWS - 16,
		kWrapWord, kAlignTextCenter);
	pd->graphics->setDrawMode(kDrawModeCopy);
}

// --- files ------------------------------------------------------------------

// The game's data folder.
void* pd_file_open_data(const char* name) { return pd->file->open(name, kFileReadData); }
// The game's own `.pdx`.
void* pd_file_open_bundled(const char* name) { return pd->file->open(name, kFileRead); }
void* pd_file_open_write(const char* name) { return pd->file->open(name, kFileWrite); }
int pd_file_read(void* file, void* buf, unsigned int len) { return pd->file->read((SDFile*)file, buf, len); }
int pd_file_write(void* file, const void* buf, unsigned int len)
{
	return pd->file->write((SDFile*)file, buf, len);
}
int pd_file_flush(void* file) { return pd->file->flush((SDFile*)file); }
int pd_file_close(void* file) { return pd->file->close((SDFile*)file); }
const char* pd_file_error(void) { return pd->file->geterr(); }
// 1 if `name` is a file in the data folder, 0 if not.
int pd_file_is_file(const char* name)
{
	FileStat stat;
	return pd->file->stat(name, &stat) == 0 && !stat.isdir;
}
int pd_file_list(const char* folder, void (*each)(const char* name, void* userdata), void* userdata)
{
	return pd->file->listfiles(folder, each, userdata, 0);
}
int pd_file_mkdir(const char* path) { return pd->file->mkdir(path); }
int pd_file_unlink(const char* name) { return pd->file->unlink(name, 0); }
int pd_file_rename(const char* from, const char* to) { return pd->file->rename(from, to); }

// --- sound ------------------------------------------------------------------

// A sound from the `.pdx`, loaded into memory; NULL if there is none.
void* pd_sample_load(const char* path) { return pd->sound->sample->load(path); }
float pd_sample_length(void* sample) { return pd->sound->sample->getLength((AudioSample*)sample); }
void* pd_sampleplayer_new(void) { return pd->sound->sampleplayer->newPlayer(); }
// Plays `sample` from its start on `player`, stopping what it played. The
// SDK's `repeat`: 1 is once, 0 loops until it is stopped.
void pd_sampleplayer_play(void* player, void* sample, float volume, int looped, float rate)
{
	SamplePlayer* p = (SamplePlayer*)player;
	pd->sound->sampleplayer->stop(p);
	pd->sound->sampleplayer->setSample(p, (AudioSample*)sample);
	pd->sound->sampleplayer->setVolume(p, volume, volume);
	pd->sound->sampleplayer->play(p, looped ? 0 : 1, rate);
}
void pd_sampleplayer_set_volume(void* player, float volume)
{
	pd->sound->sampleplayer->setVolume((SamplePlayer*)player, volume, volume);
}
void pd_sampleplayer_stop(void* player)
{
	pd->sound->sampleplayer->setPaused((SamplePlayer*)player, 0);
	pd->sound->sampleplayer->stop((SamplePlayer*)player);
}
int pd_sampleplayer_is_playing(void* player)
{
	return pd->sound->sampleplayer->isPlaying((SamplePlayer*)player);
}
void pd_sampleplayer_set_paused(void* player, int paused)
{
	pd->sound->sampleplayer->setPaused((SamplePlayer*)player, paused);
}

// A new file player on the file at `path` in the `.pdx`; NULL if there is none.
void* pd_fileplayer_open(const char* path)
{
	FilePlayer* p = pd->sound->fileplayer->newPlayer();
	if (p != NULL && !pd->sound->fileplayer->loadIntoPlayer(p, path)) {
		pd->sound->fileplayer->freePlayer(p);
		return NULL;
	}
	return p;
}
// The SDK's `repeat`, as for a sample player. After `pause` it goes on from
// where the file was.
int pd_fileplayer_play(void* player, int looped)
{
	return pd->sound->fileplayer->play((FilePlayer*)player, looped ? 0 : 1);
}
void pd_fileplayer_set_volume(void* player, float volume)
{
	pd->sound->fileplayer->setVolume((FilePlayer*)player, volume, volume);
}
void pd_fileplayer_set_rate(void* player, float rate) { pd->sound->fileplayer->setRate((FilePlayer*)player, rate); }
void pd_fileplayer_pause(void* player) { pd->sound->fileplayer->pause((FilePlayer*)player); }
void pd_fileplayer_stop(void* player) { pd->sound->fileplayer->stop((FilePlayer*)player); }
void pd_fileplayer_rewind(void* player) { pd->sound->fileplayer->setOffset((FilePlayer*)player, 0.0f); }
int pd_fileplayer_is_playing(void* player) { return pd->sound->fileplayer->isPlaying((FilePlayer*)player); }
float pd_fileplayer_length(void* player) { return pd->sound->fileplayer->getLength((FilePlayer*)player); }
void pd_fileplayer_close(void* player)
{
	pd->sound->fileplayer->stop((FilePlayer*)player);
	pd->sound->fileplayer->freePlayer((FilePlayer*)player);
}

// --- the entry point --------------------------------------------------------

// `src/device/mod.rs` and `src/sdk.rs` read these numbers.
_Static_assert(kEventLock == 2 && kEventUnlock == 3 && kEventPause == 4 && kEventResume == 5
		&& kEventTerminate == 6 && kEventLowPower == 9,
	"the event numbers in src/sdk.rs no longer match pd_api_sys.h");

static int update(void* userdata)
{
	(void)userdata;
	CALL_GAME(return shuyi_playdate_update(), 0);
}

#ifdef _WINDLL
__declspec(dllexport)
#endif
int eventHandler(PlaydateAPI* playdate, PDSystemEvent event, uint32_t arg)
{
	(void)arg;
	if (event == kEventInit) {
		const char* error = NULL;
		pd = playdate;
		font = pd->graphics->loadFont("/System/Fonts/Asheville-Sans-14-Bold.pft", &error);
		pd->system->setUpdateCallback(update, NULL);
		CALL_GAME(shuyi_playdate_init(), 0);
	} else if (event != kEventInitLua) {
		CALL_GAME(shuyi_playdate_event((int)event), 0);
	}
	return 0;
}
