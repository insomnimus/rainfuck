use std::{
	fmt::{self,},
	io,
};

#[derive(Debug)]
pub enum Error {
	RightDpOverflow { from: usize, amount: usize },
	LeftDpOverflow { from: usize, amount: usize },
	Io(io::Error),
	Oom { have: usize, want: usize },
	AddOverflow { mem: u8, value: usize },
	SubOverflow { mem: u8, value: usize },
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("runtime error: ")?;
		match self {
			Self::LeftDpOverflow { from, amount } => {
				write!(f, "data pointer moved below 0: {from} - {amount}")
			}
			Self::RightDpOverflow { from, amount } => write!(
				f,
				"data pointer exceeded the maximum possible size: {from} + {amount}"
			),
			Self::AddOverflow { mem, value } => {
				write!(f, "attempt to add with overflow: {mem} + {value}")
			}
			Self::SubOverflow { mem, value } => {
				write!(f, "attempt to subtrack with overflow: {mem} - {value}")
			}
			Self::Io(e) => write!(f, "io error: {e}"),
			Self::Oom { have, want } => write!(
				f,
				"exceeded the upper limit on memory: have {have}, want {want}"
			),
		}
	}
}

impl std::error::Error for Error {}
