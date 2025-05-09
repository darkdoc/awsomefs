package driver

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"

	"github.com/container-storage-interface/spec/lib/go/csi"
)

const baseMountPath = "/mnt/awsomefs" // your global mount

func (d *Driver) NodePublishVolume(
	ctx context.Context,
	req *csi.NodePublishVolumeRequest,
) (*csi.NodePublishVolumeResponse, error) {

	// --- Sanity check: is baseMountPath mounted? ---
	if err := checkMountpoint(baseMountPath); err != nil {
		return nil, fmt.Errorf("base mount path %s is not mounted: %v", baseMountPath, err)
	}

	volID := req.GetVolumeId()
	if volID == "" {
		return nil, fmt.Errorf("volume ID is required")
	}

	targetPath := req.GetTargetPath()
	if targetPath == "" {
		return nil, fmt.Errorf("target path is required")
	}

	sourcePath := filepath.Join(baseMountPath, "volumes", volID)

	// Check that source path exists
	if _, err := os.Stat(sourcePath); os.IsNotExist(err) {
		return nil, fmt.Errorf("volume source path %s does not exist", sourcePath)
	}

	// Make sure target path exists
	if err := os.MkdirAll(targetPath, 0755); err != nil {
		return nil, fmt.Errorf("failed to create target path: %v", err)
	}

	// Check if already mounted
	cmd := exec.Command("mountpoint", "-q", targetPath)
	if err := cmd.Run(); err == nil {
		// Already mounted
		log.Printf("Volume %s already mounted at %s", volID, targetPath)
		return &csi.NodePublishVolumeResponse{}, nil
	}

	// Perform the bind mount
	cmd = exec.Command("mount", "--bind", sourcePath, targetPath)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return nil, fmt.Errorf("failed to bind mount: %v, output: %s", err, string(output))
	}

	log.Printf("Successfully bind-mounted %s to %s", sourcePath, targetPath)
	return &csi.NodePublishVolumeResponse{}, nil
}

func (d *Driver) NodeUnpublishVolume(
	ctx context.Context,
	req *csi.NodeUnpublishVolumeRequest,
) (*csi.NodeUnpublishVolumeResponse, error) {

	// --- Sanity check: is baseMountPath mounted? ---
	if err := checkMountpoint(baseMountPath); err != nil {
		return nil, fmt.Errorf("base mount path %s is not mounted: %v", baseMountPath, err)
	}

	targetPath := req.GetTargetPath()
	if targetPath == "" {
		return nil, fmt.Errorf("target path is required")
	}

	// Check if target path is a mountpoint
	cmd := exec.Command("mountpoint", "-q", targetPath)
	if err := cmd.Run(); err != nil {
		// Not a mountpoint, assume already unmounted
		log.Printf("Target path %s is not a mountpoint or already unmounted", targetPath)
	} else {
		// Unmount it
		if err := exec.Command("umount", targetPath).Run(); err != nil {
			return nil, fmt.Errorf("failed to unmount %s: %v", targetPath, err)
		}
		log.Printf("Successfully unmounted %s", targetPath)
	}

	// Optionally remove the target directory
	if err := os.RemoveAll(targetPath); err != nil {
		return nil, fmt.Errorf("failed to remove target path %s: %v", targetPath, err)
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
