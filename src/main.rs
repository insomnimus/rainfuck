mod interp;
mod parser;
mod syntax;

use std::{
	fs::{
		self,
		File,
		OpenOptions,
	},
	io::{
		self,
		Read,
		Write,
	},
};

use clap::Parser;

use self::interp::{
	EofMode,
	Interpreter,
	Io,
	Overflow,
	OverflowOptions,
};

#[derive(Parser)]
#[command(version)]
/// Execute a brainfuck script
struct Args {
	#[arg()]
	/// The brainfuck script
	file: String,
	#[arg(short, long, default_value = "-")]
	/// The input file ("-" for stdin)
	input: String,
	#[arg(short, long, default_value = "-")]
	/// The output file ("-" for stdout)
	out: String,

	#[arg(short = 'm', long, default_value = "1000000")]
	/// The maximum memory in bytes the runtime can allocate
	max_memory: usize,
	#[arg(long, default_value = "1024")]
	/// Maximum size of the io buffer
	max_io: usize,

	#[arg(short = 'w', long, default_value = "wrap")]
	/// Arithmetic overflow mode
	overflow: Overflow,
	#[arg(short = 'W', long, default_value = "check")]
	/// Data pointer overflow mode
	ptr_overflow: Overflow,

	#[arg(short, long, default_value = "noop")]
	/// Behaviour on reading input after EOF
	eof_mode: EofMode,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
	let args = Args::parse();
	let code = fs::read(&args.file)?;
	let ops = parser::ops(&code)?;
	let mut stdout;
	let mut stdin;
	let mut out_file;
	let mut in_file;

	let output: &mut dyn Write = if args.out == "-" {
		stdout = io::stdout().lock();
		&mut stdout
	} else {
		out_file = File::create(&args.out)?;
		&mut out_file
	};

	let input: &mut dyn Read = if args.input == "-" {
		stdin = io::stdin().lock();
		&mut stdin
	} else {
		in_file = OpenOptions::new().read(true).open(&args.input)?;
		&mut in_file
	};

	let i = Interpreter::new(
		ops,
		Io { input, output },
		OverflowOptions {
			cell: args.overflow,
			ptr: args.ptr_overflow,
		},
		args.eof_mode,
		args.max_memory,
		args.max_io,
	);

	i.eval()?;
	Ok(())
}

fn main() {
	if let Err(e) = run() {
		eprintln!("error: {e:?}");
		std::process::exit(1);
	}
}
