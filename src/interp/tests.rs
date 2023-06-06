use std::io::Cursor;

use crate::{
	interp::*,
	parser,
};

fn new<O: Write>(
	script: &[u8],
	input: &'static [u8],
	output: O,
	overflow: OverflowOptions,
	eof: EofMode,
) -> Interpreter<Cursor<&'static [u8]>, O> {
	Interpreter::new(
		parser::ops(script).unwrap(),
		Io {
			input: Cursor::new(input),
			output,
		},
		overflow,
		eof,
		0,
		0,
	)
}

#[test]
fn eof_set_zero() {
	let script = b">,>+++++++++,>+++++++++++[<++++++<++++++<+>>>-]<<.>.<<-.>.>.<<.";
	let mut output = Cursor::new([0; 16]);

	new(
		script,
		b"\n",
		&mut output,
		OverflowOptions::default(),
		EofMode::Set0,
	)
	.eval()
	.unwrap();

	let expected = b"LB\nLB\n\0\0\0\0\0\0\0\0\0\0";
	assert_eq!(expected, &output.into_inner());
}

#[test]
fn eof_noop() {
	let script = b">,>+++++++++,>+++++++++++[<++++++<++++++<+>>>-]<<.>.<<-.>.>.<<.";
	let mut output = Cursor::new([0; 16]);

	new(
		script,
		b"\n",
		&mut output,
		OverflowOptions::default(),
		EofMode::Noop,
	)
	.eval()
	.unwrap();

	let expected = b"LK\nLK\n\0\0\0\0\0\0\0\0\0\0";
	assert_eq!(expected, &output.into_inner());
}

#[test]
fn cell_30k() {
	let script = b"++++[>++++++<-]>[>+++++>+++++++<<-]>>++++<[[>[[>>+
	<<-]<]>>>-]>-[>+>+<<-]>]+++++[>+++++++<<++>-]>.<<.";
	let mut output = Cursor::new([0; 4]);
	new(
		script,
		b"",
		&mut output,
		OverflowOptions::default(),
		EofMode::Set0,
	)
	.eval()
	.unwrap();

	assert_eq!(b"#\n\0\0", &output.into_inner(),);
}

#[test]
fn rot13() {
	let script = include_bytes!("rot13.b");
	let input = b"~mlk zyx";
	let expected = b"~zyx mlk\0\0";
	let mut output = Cursor::new([0; 10]);

	new(
		script,
		input,
		&mut output,
		OverflowOptions::default(),
		EofMode::Noop,
	)
	.eval()
	.unwrap();

	assert_eq!(expected, &output.into_inner());
}

#[test]
fn head() {
	let script = include_bytes!("head.b");
	let input = b"1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12n";
	let expected = b"1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
	let mut output = Cursor::new([0; 32]);
	new(
		script,
		input,
		&mut output,
		OverflowOptions::default(),
		EofMode::Noop,
	)
	.eval()
	.unwrap();

	let got = output.into_inner();
	assert_eq!(expected, &got[..expected.len()],);

	assert!(got[expected.len()..].iter().all(|&b| b == 0),);
}
