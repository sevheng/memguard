.PHONY: all deb rpm clean

all:
	cargo build --release

deb: all
	cargo deb --no-build
	cargo deb --no-build --variant system-tune

rpm: all
	cargo generate-rpm
	cargo generate-rpm --variant system-tune

clean:
	cargo clean
	rm -rf target/debian target/generate-rpm
