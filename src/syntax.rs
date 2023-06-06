use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Token {
	Left,
	Right,
	Add,
	Sub,
	Read,
	Write,
	LBracket,
	RBracket,
}

impl fmt::Display for Token {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match *self {
			Self::Left => "<",
			Self::Right => ">",
			Self::Add => "+",
			Self::Sub => "-",
			Self::Read => ",",
			Self::Write => ".",
			Self::LBracket => "[",
			Self::RBracket => "]",
		})
	}
}
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct TokenSpan {
	pub token: Token,
	pub index: usize,
}

pub struct Tokens<'a> {
	code: &'a [u8],
	pos: usize,
}

impl<'a> Tokens<'a> {
	pub fn new(code: &'a [u8]) -> Self {
		Self { code, pos: 0 }
	}
}

impl<'a> Iterator for Tokens<'a> {
	type Item = TokenSpan;

	fn next(&mut self) -> Option<TokenSpan> {
		if self.pos >= self.code.len() {
			return None;
		}

		let t = self.code[self.pos..]
			.iter()
			.enumerate()
			.filter_map(|(i, &b)| {
				Some(TokenSpan {
					index: self.pos + i,
					token: match b {
						b'<' => Token::Left,
						b'>' => Token::Right,
						b'+' => Token::Add,
						b'-' => Token::Sub,
						b',' => Token::Read,
						b'.' => Token::Write,
						b'[' => Token::LBracket,
						b']' => Token::RBracket,
						_ => return None,
					},
				})
			})
			.next()?;

		self.pos = t.index + 1;
		Some(t)
	}
}
