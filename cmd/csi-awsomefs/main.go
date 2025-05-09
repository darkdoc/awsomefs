package main

import (
	"flag"
	"log"

	"csi-awsomefs/driver"
)

func main() {
	var (
		endpoint = flag.String("endpoint", "unix:///csi/csi.sock", "CSI endpoint")
		mode     = flag.String("mode", "node", "CSI mode")
	)
	flag.Parse()
	d, err := driver.NewDriver("awsomefs.csi.driver", "0.1.0", *endpoint)
	if err != nil {
		log.Fatalf("Failed to create driver: %v", err)
	}

	if err := d.Run(*mode); err != nil {
		log.Fatalf("Failed to run driver: %v", err)
	}
}
