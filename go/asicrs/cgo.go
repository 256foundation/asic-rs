package asicrs

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib -lasic_rs_ffi
#cgo darwin LDFLAGS: -Wl,-rpath,${SRCDIR}/lib -framework Security -framework CoreFoundation
#cgo linux LDFLAGS: -Wl,-rpath,${SRCDIR}/lib -lm -ldl -lpthread
#cgo windows LDFLAGS: -lws2_32 -luserenv -lbcrypt -lntdll

#include "asic_rs_ffi.h"
#include <stdlib.h>
*/
import "C"

import (
	"encoding/json"
	"errors"
	"fmt"
	"strings"
	"unsafe"
)

// ErrNotFound is returned by Factory.GetMiner and Factory.ScanMiner when the
// address did not identify as a supported ASIC miner.
var ErrNotFound = errors.New("no supported miner found")

var errInteriorNUL = errors.New("string contains interior NUL")

func lastError() error {
	cstr := C.asic_rs_last_error()
	if cstr == nil {
		return errors.New("unknown asic-rs error")
	}
	return errors.New(C.GoString(cstr))
}

func freeCString(s *C.char) {
	if s != nil {
		C.asic_rs_free_string(s)
	}
}

func goStringOwned(s *C.char) string {
	if s == nil {
		return ""
	}
	defer freeCString(s)
	return C.GoString(s)
}

func takeJSON(s *C.char, dest any) error {
	if s == nil {
		return lastError()
	}
	defer freeCString(s)
	if err := json.Unmarshal([]byte(C.GoString(s)), dest); err != nil {
		return fmt.Errorf("decode json: %w", err)
	}
	return nil
}

func takeJSONBytes(s *C.char) ([]byte, error) {
	if s == nil {
		return nil, lastError()
	}
	defer freeCString(s)
	return []byte(C.GoString(s)), nil
}

func cString(s string) (*C.char, error) {
	if strings.IndexByte(s, 0) >= 0 {
		return nil, errInteriorNUL
	}
	return C.CString(s), nil
}

func freeGoCString(s *C.char) {
	if s != nil {
		C.free(unsafe.Pointer(s))
	}
}

func controlResult(code C.int32_t) (bool, error) {
	switch code {
	case 1:
		return true, nil
	case 0:
		return false, nil
	default:
		return false, lastError()
	}
}

func lookupResult(code C.int32_t, ptr *C.AsicMiner) (*Miner, error) {
	switch code {
	case 0:
		return newMiner(ptr), nil
	case 1:
		return nil, ErrNotFound
	default:
		return nil, lastError()
	}
}
