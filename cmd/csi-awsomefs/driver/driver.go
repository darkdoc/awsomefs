package driver

import (
	"net"

	"github.com/container-storage-interface/spec/lib/go/csi"
	"google.golang.org/grpc"
	"google.golang.org/grpc/reflection"
)

type Driver struct {
	name    string
	version string

	csi.UnimplementedIdentityServer
	csi.UnimplementedNodeServer
}

func NewDriver(name, version string) (*Driver, error) {
	return &Driver{name: name, version: version}, nil
}

func (d *Driver) Run() error {
	server := grpc.NewServer()

	csi.RegisterIdentityServer(server, d)
	csi.RegisterNodeServer(server, d)

	reflection.Register(server)

	listener, err := net.Listen("tcp", ":10000")
	if err != nil {
		return err
	}

	return server.Serve(listener)
}
