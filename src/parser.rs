use std::{
	fmt::{self,},
	num::NonZeroUsize,
};

use crate::syntax::{
	Token,
	TokenSpan,
	Tokens,
};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ErrorKind {
	UnmatchedBracket,
	UnexpectedBracket,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Error {
	kind: ErrorKind,
	offset: usize,
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", self)
	}
}

impl std::error::Error for Error {}

impl Error {
	fn unexpected_bracket(offset: usize, _: &[u8]) -> Self {
		Self {
			offset,
			kind: ErrorKind::UnexpectedBracket,
		}
	}

	fn unmatched_bracket(offset: usize, _: &[u8]) -> Self {
		Self {
			offset,
			kind: ErrorKind::UnmatchedBracket,
		}
	}
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Op {
	pub t: Token,
	pub n: NonZeroUsize,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct Ir {
	t: TokenSpan,
	n: usize,
}

fn collapse(code: &[u8]) -> Vec<Ir> {
	let mut tokens = Tokens::new(code).peekable();
	let mut irs = Vec::new();
	// Collapse adjacent duplicate tokens
	while let Some(t) = tokens.next() {
		match t.token {
			Token::LBracket | Token::RBracket => irs.push(Ir { t, n: 0 }),
			_ => {
				let mut n = 1;
				while tokens.next_if(|next| next.token == t.token).is_some() {
					n += 1;
				}
				irs.push(Ir { t, n })
			}
		}
	}

	irs
}

fn calculate_jmp(irs: &mut [Ir]) -> usize {
	debug_assert_eq!(
		irs[0].t.token,
		Token::LBracket,
		"calculate_jmp called on a token other than LBracket"
	);

	let mut i = 1;
	while i < irs.len() {
		let ir = irs[i];
		match ir.t.token {
			Token::RBracket => {
				irs[i].n = i;
				irs[0].n = i;
				return i;
			}
			Token::LBracket => {
				// We need to update the jump values
				let stepped = calculate_jmp(&mut irs[i..]);
				if stepped == 0 {
					return 0;
				}
				i += stepped;
			}
			_ => (),
		}

		i += 1;
	}

	0
}

pub fn ops(code: &[u8]) -> Result<Vec<Op>, Error> {
	let mut irs = collapse(code);
	let mut i = 0;

	while i < irs.len() {
		let ir = irs[i];
		match ir.t.token {
			Token::RBracket if ir.n == 0 => {
				return Err(Error::unexpected_bracket(ir.t.index, code))
			}
			Token::LBracket if ir.n == 0 => {
				// We need to update the jump values
				let stepped = calculate_jmp(&mut irs[i..]);
				if stepped == 0 {
					return Err(Error::unmatched_bracket(ir.t.index, code));
				}
				i += stepped;
			}
			_ => (),
		}

		i += 1;
	}

	let mut cutoff = 0;
	// Do some optimizations
	if irs[0].t.token == Token::LBracket {
		// If [] is the first op, it's guaranteed to be never run
		cutoff = irs[0].n + 1;
	}

	Ok(irs
		.into_iter()
		.skip(cutoff)
		.map(|x| Op {
			t: x.t.token,
			n: NonZeroUsize::new(x.n).expect("assertion failed"),
		})
		.collect())
}
