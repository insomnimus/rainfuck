mod error;
#[cfg(test)]
mod tests;

use std::{
	io::{
		self,
		Read,
		Write,
	},
	num::NonZeroUsize,
};

pub use self::error::Error;
use crate::{
	parser::Op,
	syntax::Token,
};

type Result<T> = ::std::result::Result<T, Error>;

#[derive(clap::ValueEnum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum Overflow {
	/// Overflows wrap around
	Wrap,
	/// Overflows saturate
	Saturate,
	/// Overflows cause an unrecoverable error
	Check,
}

#[derive(clap::ValueEnum, Copy, Clone, Eq, PartialEq, Debug)]
pub enum EofMode {
	/// Reads after EOF do nothing
	Noop,
	/// Reads after EOF set the cell to 0
	Set0,
	/// Reads after EOF terminate the program
	Terminate,
}

impl From<io::Error> for Error {
	fn from(e: io::Error) -> Self {
		Self::Io(e)
	}
}

pub struct Interpreter<I, O> {
	ip: usize,
	dp: usize,
	mem: Vec<u8>,
	ops: Vec<Op>,
	iobuf: Vec<u8>,

	input: I,
	output: O,

	cell_overflow_mode: Overflow,
	ptr_overflow_mode: Overflow,
	eof_mode: EofMode,
	max_mem: usize,
	max_io: usize,
	reached_eof: bool,
}

pub struct Io<I, O> {
	pub input: I,
	pub output: O,
}

pub struct OverflowOptions {
	pub cell: Overflow,
	pub ptr: Overflow,
}

impl Default for OverflowOptions {
	fn default() -> Self {
		Self {
			cell: Overflow::Wrap,
			ptr: Overflow::Check,
		}
	}
}

impl<I: Read, O: Write> Interpreter<I, O> {
	pub fn new<C: IntoIterator<Item = Op>>(
		ops: C,
		io: Io<I, O>,
		overflow: OverflowOptions,
		eof_mode: EofMode,
		max_mem: usize,
		max_io: usize,
	) -> Self {
		Self {
			dp: 0,
			ip: 0,
			mem: vec![0; 32 << 10],
			ops: ops.into_iter().collect(),
			input: io.input,
			output: io.output,
			iobuf: Vec::with_capacity(usize::min(max_io, 128)),
			cell_overflow_mode: overflow.cell,
			ptr_overflow_mode: overflow.ptr,
			max_io: usize::max(max_io, 4),
			max_mem: usize::max(max_mem, 32 << 10),
			eof_mode,
			reached_eof: false,
		}
	}

	pub fn eval(mut self) -> Result<()> {
		while self.ip < self.ops.len() {
			let op = self.ops[self.ip];
			match op.t {
				Token::Add => self.add(op.n)?,
				Token::Sub => self.sub(op.n)?,
				Token::Left => self.left(op.n)?,
				Token::Right => {
					self.dp = self
						.dp
						.checked_add(op.n.get())
						.ok_or(Error::RightDpOverflow {
							from: self.dp,
							amount: op.n.get(),
						})?
				}
				Token::Read => {
					self.ensure_mem()?;
					self.mem[self.dp] = self.read(op.n)?;
				}
				Token::Write => {
					let val = *self.mem.get(self.dp).unwrap_or(&0);

					let bytes = [val; 16];
					let chunks = op.n.get() / 16;
					let rem = op.n.get() % 16;
					for _ in 0..chunks {
						self.output.write(&bytes).map_err(Error::Io)?;
					}
					if rem > 0 {
						self.output.write(&bytes[..rem]).map_err(Error::Io)?;
					}
				}
				Token::LBracket => {
					if self.mem.get(self.dp).map_or(true, |&n| n == 0) {
						self.ip += op.n.get();
					}
				}
				Token::RBracket => {
					if self.mem.get(self.dp).map_or(false, |&b| b != 0) {
						self.ip -= op.n.get();
					}
				}
			}

			self.ip += 1;
		}

		Ok(())
	}

	#[inline(always)]
	fn value(&mut self) -> &mut u8 {
		&mut self.mem[self.dp]
	}

	fn read(&mut self, times: NonZeroUsize) -> Result<u8> {
		let times = times.get();
		// ensure capacity
		let need = usize::min(self.max_io, times).saturating_sub(self.iobuf.len());
		self.iobuf.reserve(need);
		for _ in 0..need {
			self.iobuf.push(0);
		}

		if self.reached_eof {
			return match self.eof_mode {
				EofMode::Noop => Ok(*self.value()),
				EofMode::Set0 => Ok(0),
				EofMode::Terminate => Err(Error::Io(io::Error::new(
					io::ErrorKind::UnexpectedEof,
					"reached end of input but a a read command was executed",
				))),
			};
		}

		let mut remaining_bytes = times;
		let mut last_read = *self.value();

		while remaining_bytes > 0 {
			let n = self.input.read(&mut self.iobuf)?;
			if n == 0 {
				self.reached_eof = true;
				return match self.eof_mode {
					EofMode::Noop => Ok(last_read),
					EofMode::Set0 => Ok(0),
					EofMode::Terminate => Err(Error::Io(io::Error::new(
						io::ErrorKind::UnexpectedEof,
						"reached end of input but a a read command was executed",
					))),
				};
			}
			last_read = self.iobuf[n - 1];
			remaining_bytes -= n;
		}

		Ok(last_read)
	}

	fn add(&mut self, times: NonZeroUsize) -> Result<()> {
		self.ensure_mem()?;
		let times = times.get();

		match self.cell_overflow_mode {
			Overflow::Wrap => {
				let n = self.value().wrapping_add((times % 256) as u8);
				*self.value() = n;
			}
			Overflow::Saturate => {
				let n = (*self.value() as usize).saturating_add(times);
				*self.value() = usize::min(255, n) as u8;
			}
			Overflow::Check => {
				let mem = self.value();
				let n = u8::try_from(times).map_err(|_| Error::AddOverflow {
					mem: *mem,
					value: times,
				})?;
				*mem = mem.checked_add(n).ok_or(Error::AddOverflow {
					mem: *mem,
					value: times,
				})?;
			}
		}

		Ok(())
	}

	fn sub(&mut self, times: NonZeroUsize) -> Result<()> {
		self.ensure_mem()?;
		let times = times.get();

		match self.cell_overflow_mode {
			Overflow::Wrap => {
				let mem = self.value();
				let n = (*mem as usize).wrapping_sub(times);
				*mem = (n % 256) as u8;
			}
			Overflow::Check => {
				let mem = self.value();
				let n = (*mem as usize)
					.checked_sub(times)
					.ok_or(Error::SubOverflow {
						mem: *mem,
						value: times,
					})?;
				*mem = u8::try_from(n).map_err(|_| Error::SubOverflow {
					mem: *mem,
					value: times,
				})?;
			}
			Overflow::Saturate => {
				let mem = self.value();
				if times >= 255 || times as u8 >= *mem {
					*mem = 0;
				} else {
					*mem -= times as u8;
				}
			}
		}

		Ok(())
	}

	fn ensure_mem(&mut self) -> Result<()> {
		if self.dp >= self.max_mem {
			return Err(Error::Oom {
				have: self.max_mem,
				want: self.dp,
			});
		}

		let need = usize::min(self.dp + 1, self.max_mem).saturating_sub(self.mem.len());
		self.mem.reserve(need);
		for _ in 0..need {
			self.mem.push(0);
		}

		Ok(())
	}

	fn left(&mut self, times: NonZeroUsize) -> Result<()> {
		let times = times.get();
		match self.ptr_overflow_mode {
			Overflow::Saturate => self.dp = self.dp.saturating_sub(times),
			Overflow::Wrap => {
				if self.dp >= times {
					self.dp -= times;
				} else {
					// let n = times % self.mem.len() + self.mem.len();
					let n = times % self.max_mem + self.max_mem;
					self.dp = n - self.dp;
					if self.dp == self.max_mem { self.dp = 0; }
				}
			}
			Overflow::Check => {
				self.dp = self.dp.checked_sub(times).ok_or(Error::LeftDpOverflow {
					from: self.dp,
					amount: times,
				})?
			}
		}

		Ok(())
	}
}
