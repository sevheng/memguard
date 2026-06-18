.PHONY: all deb rpm clean ananicy-cpp

all:
	cargo build --release

ananicy-cpp:
	bash system-tune/build-ananicy-cpp.sh

deb: all ananicy-cpp
	cargo deb --no-build
	cargo deb --no-build --variant system-tune

rpm: all ananicy-cpp
	cargo generate-rpm
	cargo generate-rpm --variant system-tune

clean:
	cargo clean
	rm -rf target/debian target/generate-rpm
	rm -rf system-tune/ananicy-cpp/.build system-tune/ananicy-cpp/ananicy-cpp
