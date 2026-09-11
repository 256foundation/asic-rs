// Example: discover a miner by IP and print a telemetry snapshot.
//
//	ASIC_MINER_IP=192.168.1.42 go run ./examples/get_data
package main

import (
	"errors"
	"fmt"
	"log"
	"os"

	"github.com/256foundation/asic-rs/go/asicrs"
)

func main() {
	ip := os.Getenv("ASIC_MINER_IP")
	if ip == "" {
		ip = "192.168.1.10"
		fmt.Fprintf(os.Stderr, "ASIC_MINER_IP not set; trying %s\n", ip)
	}

	fmt.Println("asic-rs-ffi version:", asicrs.Version())

	factory := asicrs.NewFactory().
		WithIdentificationTimeoutSecs(8).
		WithPortCheck(true)
	defer factory.Close()

	miner, err := factory.GetMiner(ip)
	if errors.Is(err, asicrs.ErrNotFound) {
		log.Fatalf("no supported miner at %s", ip)
	}
	if err != nil {
		log.Fatalf("get miner at %s: %v", ip, err)
	}
	defer miner.Close()

	summary, _ := miner.Summary()
	fmt.Println("found:", summary)

	caps, _ := miner.Supports()
	fmt.Printf("supports: restart=%v pause=%v pools=%v fan=%v\n",
		caps.Restart, caps.Pause, caps.PoolsConfig, caps.FanConfig)

	data, err := miner.GetData()
	if err != nil {
		log.Fatalf("get data: %v", err)
	}

	fmt.Printf("model=%s firmware=%s mining=%v hashrate=%.2f TH/s\n",
		data.DeviceInfo.Model,
		data.DeviceInfo.Firmware,
		data.IsMining,
		data.HashrateTH(),
	)
	if data.Wattage != nil {
		fmt.Printf("wattage=%.0f W\n", *data.Wattage)
	}
	if data.AverageTemperature != nil {
		fmt.Printf("avg_temp=%.1f C\n", *data.AverageTemperature)
	}
	if data.BestShare != nil {
		fmt.Printf("best_share=%.0f\n", *data.BestShare)
	}
}
