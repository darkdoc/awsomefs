package driver

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"strings"

	"github.com/container-storage-interface/spec/lib/go/csi"
)

const baseMountPath = "/mnt/awsomefs" // your global mount

func (d *Driver) NodePublishVolume(
	ctx context.Context,
	req *csi.NodePublishVolumeRequest,
) (*csi.NodePublishVolumeResponse, error) {

	volID := req.GetVolumeId()
	targetPath := req.GetTargetPath()

	sourcePath := fmt.Sprintf("%s/%s", baseMountPath, volID)

	// Check that source path exists
	if _, err := os.Stat(sourcePath); os.IsNotExist(err) {
		return nil, fmt.Errorf("volume source path %s does not exist", sourcePath)
	}

	// Make sure target path exists
	if err := os.MkdirAll(targetPath, 0755); err != nil {
		return nil, fmt.Errorf("failed to create target path: %v", err)
	}

	// Check if already mounted
	cmd := exec.Command("mount", "--bind", sourcePath, targetPath)
	output, err := cmd.CombinedOutput()
	if err != nil {
		// If the error is because it's already mounted, it's fine
		if strings.Contains(string(output), "already mounted") {
			fmt.Println("Volume already mounted.")
		} else {
			return nil, fmt.Errorf("failed to bind mount: %v, output: %s", err, string(output))
		}
	}

	return &csi.NodePublishVolumeResponse{}, nil
}

func (d *Driver) NodeUnpublishVolume(
	ctx context.Context,
	req *csi.NodeUnpublishVolumeRequest,
) (*csi.NodeUnpublishVolumeResponse, error) {

	targetPath := req.GetTargetPath()

	// Unmount volume
	cmd := exec.Command("umount", targetPath)
	output, err := cmd.CombinedOutput()
	if err != nil {
		// Check if it's already unmounted
		if strings.Contains(string(output), "not mounted") {
			fmt.Println("Volume is already unmounted.")
		} else {
			return nil, fmt.Errorf("failed to unmount: %v, output: %s", err, string(output))
		}
	}

	return &csi.NodeUnpublishVolumeResponse{}, nil
}

func (d *Driver) NodeGetInfo(ctx context.Context, req *csi.NodeGetInfoRequest) (*csi.NodeGetInfoResponse, error) {
	// Return basic node info (static string for now)
	return &csi.NodeGetInfoResponse{
		NodeId: "awesomefs-node-1", // static for now #TODO make this dynamic
	}, nil
}

func (d *Driver) NodeGetCapabilities(ctx context.Context, req *csi.NodeGetCapabilitiesRequest) (*csi.NodeGetCapabilitiesResponse, error) {
	return &csi.NodeGetCapabilitiesResponse{
		Capabilities: []*csi.NodeServiceCapability{
			// Can be expanded later if necessary
		},
	}, nil
}
