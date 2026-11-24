//! The agent eval of the end goal (ticket 2524): games an agent is asked
//! to make, as data ([`task`]), graded through the tool contract and cargo
//! alone ([`grade`]), and the reference solutions CI grades on every pull
//! request ([`reference`]). `evals/README.md` says what it measures.

pub mod grade;
pub mod reference;
pub mod task;

#[cfg(test)]
mod tests;
