.PHONY: all ffi test examples tidy fmt clean

PROFILE ?= debug
ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
FFI := $(ROOT)/asic-rs-ffi
OUT_INC := $(abspath asicrs/include)
OUT_LIB := $(abspath asicrs/lib)

ifeq ($(PROFILE),release)
CARGO_FLAGS := --release
TARGET_DIR := $(ROOT)/target/release
else
CARGO_FLAGS :=
TARGET_DIR := $(ROOT)/target/debug
endif

all: ffi test

ffi:
	cargo build -p asic-rs-ffi $(CARGO_FLAGS)
	mkdir -p "$(OUT_INC)" "$(OUT_LIB)"
	cp "$(FFI)/include/asic_rs_ffi.h" "$(OUT_INC)/asic_rs_ffi.h"
	@copied=0; \
	for ext in a dylib so dll lib; do \
	  for f in "$(TARGET_DIR)"/libasic_rs_ffi.$$ext "$(TARGET_DIR)"/asic_rs_ffi.$$ext; do \
	    if [ -f "$$f" ]; then \
	      cp "$$f" "$(OUT_LIB)/"; \
	      echo "installed $$(basename "$$f")"; \
	      copied=1; \
	    fi; \
	  done; \
	done; \
	if [ "$$copied" -eq 0 ]; then \
	  echo "error: no libasic_rs_ffi found in $(TARGET_DIR)" >&2; \
	  exit 1; \
	fi

test: ffi
	CGO_ENABLED=1 go test ./asicrs/ -count=1

examples: ffi
	CGO_ENABLED=1 go build ./examples/get_data ./examples/scan

tidy:
	go mod tidy

fmt:
	gofmt -w asicrs examples
	cargo fmt -p asic-rs-ffi

clean:
	rm -f asicrs/lib/libasic_rs_ffi.* asicrs/lib/asic_rs_ffi.*
	cargo clean -p asic-rs-ffi
