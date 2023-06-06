# Rainfuck
Rainfuck is a [brainfuck](https://en.wikipedia.org/wiki/Brainfuck) interpreter.

## Features
- Configurable add/sub overflow modes: wrap, saturate, check (runtime error)
- Configurable pointer overflow modes: wrap around existing memory, saturate, check (runtime error)
- Configurable read behaviour after EOF is reached: noop, set0, check (runtime error)

## Build Instructions
You'll need a recent [rust](https://www.rust-lang.org) toolchain.

```sh
cargo build --release
```

## How To Use
Give rainfuck the path of a brainfuck script.

```sh
rainfuck ./foo.b
```

For help about configuring the runtime behaviour like overflow modes and memory, run with `--help`.

## Example Scripts
You can find some sample brainfuck scripts in the [brainfuck_examples](brainfuck_examples/) directory.

```sh
# this prints onanan
echo "banana" | rainfuck brainfuck_examples/rot13.b
# this prints banana
echo "onanan" | rainfuck brainfuck_examples/rot13.b
```

You can also bootstrap yourself a brainfuck compiler using the awesome [matslina/awib](https://github.com/matslina/awib) which is itself in brainfuck!

As of the day of writing, the instructions are as follows:

```sh
git clone --depth 1 https://github.com/matslina/awib
cd awib
make
rainfuck awib.b -i awib.b -o awib.c # this transpiles itself to C
gcc -O2 -o awib awib.c # on POSIX platforms

# awib can compile directly into a 32 bit linux executable
# we have to add a magic line in the beginning of the script
# below works on bash (be mindful of spaces around {}, they're needed)
{ echo "@386_linux"; cat awib.b; } | rainfuck awib.b -o awib
chmod +x awib

# Use awib to compile brainfuck scripts
# For example, it can compile itself again
{ echo "@386_linux"; cat awib.b; } | ./awib > awib2
```

You can find more examples at [brainfuck.org](www.brainfuck.org).
