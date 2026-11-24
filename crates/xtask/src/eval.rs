//! The agent eval of the end goal (ticket 2524): games an agent is asked
//! to make, as data ([`task`]), graded through the tool contract and cargo
//! alone ([`grade`]). `evals/README.md` says what it measures.

pub mod grade;
pub mod task;

#[cfg(test)]
mod tests;
