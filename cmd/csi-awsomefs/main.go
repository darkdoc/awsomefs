package main

import (
	"log"

	"csi-awsomefs/driver"
)

func main() {
	d, err := driver.NewDriver("awsomefs.csi.driver", "0.1.0")
	if err != nil {
		log.Fatalf("Failed to create driver: %v", err)
	}

	if err := d.Run(); err != nil {
		log.Fatalf("Failed to run driver: %v", err)
	}
}
