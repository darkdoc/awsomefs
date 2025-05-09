package driver

import (
	"fmt"
	"log"
	"net"
	"os"
	"strings"

	"github.com/container-storage-interface/spec/lib/go/csi"
	"google.golang.org/grpc"
	"google.golang.org/grpc/reflection"
)

type Driver struct {
	name     string
	version  string
	endpoint string

	csi.UnimplementedIdentityServer
	csi.UnimplementedNodeServer
	csi.UnimplementedControllerServer
}

func NewDriver(name, version, endpoint string) (*Driver, error) {
	return &Driver{name: name, version: version, endpoint: endpoint}, nil
}

func (d *Driver) Run(mode string) error {
	network, addr, err := parseEndpoint(d.endpoint)
	if err != nil {
		return fmt.Errorf("invalid endpoint: %v", err)
	}

	if network == "unix" {
		if err := os.RemoveAll(addr); err != nil {
			return err
		}
	}

	server := grpc.NewServer()
	csi.RegisterIdentityServer(server, d)

	mode = strings.ToLower(mode)
	log.Printf("Driver starting in mode: %s", mode)

	switch mode {
	case "controller":
		csi.RegisterControllerServer(server, d)
	case "node":
		csi.RegisterNodeServer(server, d)
	default:
		log.Fatalf("invalid mode: %s", mode)
	}

	reflection.Register(server)

	listener, err := net.Listen(network, addr)
	if err != nil {
		return fmt.Errorf("failed to listen on %s: %v", d.endpoint, err)
	}
	log.Printf("Listening on: %s %s", network, addr)

	return server.Serve(listener)
}

func parseEndpoint(ep string) (string, string, error) {
	if strings.HasPrefix(ep, "unix://") {
		return "unix", strings.TrimPrefix(ep, "unix://"), nil
	}
	if strings.HasPrefix(ep, "tcp://") {
		return "tcp", strings.TrimPrefix(ep, "tcp://"), nil
	}
	return "", "", fmt.Errorf("unsupported protocol (use unix:// or tcp://)")
}
