.PHONY: all
all: main.exe

main.exe: *.rs
	rustc -o $@ main.rs

main.wasm: *.rs
	rustc --target wasm32-unknown-unknown -o $@ main.rs
