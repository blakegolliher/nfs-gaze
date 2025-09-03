# nfs-gaze

A command-line tool to monitor NFS client I/O statistics on Linux.

## Usage

```bash
# Monitor all NFS mounts in nfsiostat format
./nfs-gaze --nfsiostat

# Monitor a specific mount point with bandwidth stats
./nfs-gaze -m /mnt/nfs -bw

# Monitor specific operations (READ,WRITE) on a mount point
./nfs-gaze -m /mnt/nfs -ops READ,WRITE

# Clear the screen between updates
./nfs-gaze -m /mnt/nfs --clear
```

## Testing

To run the tests, use the following command:

```bash
go test ./...
```

To generate a test coverage report, use the following command:

```bash
go test -coverprofile=coverage.out ./... && go tool cover -html=coverage.out
```