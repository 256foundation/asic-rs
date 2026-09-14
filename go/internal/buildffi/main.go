// Build and install the artifacts reported by Cargo, including custom target directories.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
)

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func run() error {
	root := flag.String("root", "..", "Rust workspace root")
	profile := flag.String("profile", "debug", "debug or release")
	flag.Parse()
	args := []string{"build", "-p", "asic-rs-ffi", "--locked", "--message-format=json-render-diagnostics"}
	switch *profile {
	case "release":
		args = append(args, "--release")
	case "debug":
	default:
		return fmt.Errorf("unsupported profile %q", *profile)
	}
	cmd := exec.Command("cargo", args...)
	cmd.Dir = *root
	cmd.Stderr = os.Stderr
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return err
	}
	if err := cmd.Start(); err != nil {
		return err
	}
	var artifacts []string
	decoder := json.NewDecoder(stdout)
	for {
		var message struct {
			Reason string `json:"reason"`
			Target struct {
				Name string `json:"name"`
			} `json:"target"`
			Filenames []string `json:"filenames"`
		}
		err := decoder.Decode(&message)
		if err == io.EOF {
			break
		}
		if err != nil {
			// Drain before waiting so Cargo cannot block on a full output pipe.
			_, _ = io.Copy(io.Discard, stdout)
			_ = cmd.Wait()
			return fmt.Errorf("decode Cargo artifacts: %w", err)
		}
		if message.Reason == "compiler-artifact" && message.Target.Name == "asic_rs_ffi" {
			for _, path := range message.Filenames {
				switch filepath.Ext(path) {
				case ".a", ".so", ".dylib", ".dll", ".lib":
					artifacts = append(artifacts, path)
				}
			}
		}
	}
	if err := cmd.Wait(); err != nil {
		return err
	}
	if len(artifacts) == 0 {
		return fmt.Errorf("Cargo reported no asic-rs-ffi libraries")
	}
	output := filepath.Join(*root, "go", "asic_go")
	if err := install(filepath.Join(*root, "asic-rs-ffi", "include", "asic_rs_ffi.h"), filepath.Join(output, "include")); err != nil {
		return err
	}
	for _, path := range artifacts {
		if err := install(path, filepath.Join(output, "lib")); err != nil {
			return err
		}
	}
	return nil
}

func install(source, directory string) error {
	if err := os.MkdirAll(directory, 0755); err != nil {
		return err
	}
	data, err := os.ReadFile(source)
	if err != nil {
		return err
	}
	dest := filepath.Join(directory, filepath.Base(source))
	if err := os.WriteFile(dest, data, 0644); err != nil {
		return err
	}
	fmt.Println("installed", dest)
	return nil
}
