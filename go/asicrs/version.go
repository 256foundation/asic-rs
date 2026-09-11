package asicrs

/*
#include "asic_rs_ffi.h"
*/
import "C"

// Version returns the asic-rs-ffi library version (static string; no free needed).
func Version() string {
	return C.GoString(C.asic_rs_version())
}
