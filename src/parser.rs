use std::{
	fmt::{
		self,
		Write,
	},
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

pub struct Error {
	kind: ErrorKind,
	line: usize,
	col: usize,
	arrow: usize,
	code: Box<str>,
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"syntax error at line {line}, column {col}: {error}\n\t{code}\n\t",
			line = self.line,
			col = self.col,
			error = match self.kind {
				ErrorKind::UnmatchedBracket => "missing closing bracket ']'",
				ErrorKind::UnexpectedBracket => "unexpected closing bracket ']'",
			},
			code = self.code,
		)?;

		for _ in 0..self.arrow {
			f.write_char('-')?;
		}
		f.write_char('^')?;
		for _ in self.arrow + 1..self.code.len() {
			f.write_char('-')?;
		}
		Ok(())
	}
}

impl fmt::Debug for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SyntaxError")
			.field("kind", &self.kind)
			.field("line", &self.line)
			.field("col", &self.col)
			.field("code", &&self.code)
			.finish()
	}
}

impl std::error::Error for Error {}

impl Error {
	fn new(kind: ErrorKind, pos: usize, code: &[u8]) -> Self {
		let line = 1 + code[..pos].iter().filter(|&&c| c == b'\n').count();
		let left = code[..pos].iter().rposition(|&c| c == b'\n').unwrap_or(0);

		let right = pos
			+ code[pos..]
				.iter()
				.position(|&c| c == b'\n')
				.unwrap_or(code.len() - pos);
		let col = 1 + pos - left;

		// calculate the arrow for the error message.
		// replace \t with 4 spaces
		let mut buf = Vec::with_capacity(12 + right - left);
		let code = &code[left..right];
		let mut i = code
			.iter()
			.position(|c| !c.is_ascii_whitespace())
			.unwrap_or(0);
		let code = &code[i..];
		let mut arrow = 0;
		for &c in code {
			if i + left == pos {
				// we're at the token that caused the error
				arrow = buf.len();
			}
			if c == b'\t' {
				buf.extend_from_slice(b"    ");
			} else {
				buf.push(c);
			}
			i += 1;
		}

		let last_space = buf
			.iter()
			.rposition(|&c| !c.is_ascii_whitespace())
			.map(|n| n + 1)
			.unwrap_or(buf.len());

		Self {
			kind,
			line,
			col,
			arrow,
			code: String::from_utf8_lossy(&buf[..last_space])
				.into_owned()
				.into_boxed_str(),
		}
	}

	fn unexpected_bracket(pos: usize, code: &[u8]) -> Self {
		Self::new(ErrorKind::UnexpectedBracket, pos, code)
	}

	fn unmatched_bracket(pos: usize, code: &[u8]) -> Self {
		Self::new(ErrorKind::UnmatchedBracket, pos, code)
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
	if irs.is_empty() {
		return Ok(Vec::new());
	}
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
