.PHONY: all ffi test examples tidy fmt clean

PROFILE ?= debug
ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
all: ffi test

ffi:
	go run ./internal/buildffi -root "$(ROOT)" -profile "$(PROFILE)"

test: ffi
	CGO_ENABLED=1 go test ./asic_go/ -count=1

examples: ffi
	CGO_ENABLED=1 go build ./examples/get_data ./examples/scan

tidy:
	go mod tidy

fmt:
	gofmt -w asic_go examples
	cargo fmt -p asic-rs-ffi

clean:
	rm -f asic_go/lib/libasic_rs_ffi.* asic_go/lib/asic_rs_ffi.*
	cargo clean -p asic-rs-ffi
