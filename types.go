package main

// NFSOperation holds statistics for a single NFS operation
type NFSOperation struct {
	Name        string
	Ops         int64
	Ntrans      int64
	Timeouts    int64
	BytesSent   int64
	BytesRecv   int64
	QueueTime   int64 // in milliseconds
	RTT         int64 // in milliseconds
	ExecuteTime int64 // in milliseconds
	Errors      int64
}

// NFSEvents holds event statistics
type NFSEvents struct {
	InodeRevalidate  int64 // index 0
	DentryRevalidate int64 // index 1
	DataInvalidate   int64 // index 2
	AttrInvalidate   int64 // index 3
	VFSOpen          int64 // index 4
	VFSLookup        int64 // index 5
	VFSAccess        int64 // index 6
	VFSUpdatePage    int64 // index 7
	VFSReadPage      int64 // index 8
	VFSReadPages     int64 // index 9
	VFSWritePage     int64 // index 10
	VFSWritePages    int64 // index 11
	VFSGetdents      int64 // index 12
	VFSSetattr       int64 // index 13
	VFSFlush         int64 // index 14
	VFSFsync         int64 // index 15
	VFSLock          int64 // index 16
	VFSRelease       int64 // index 17
	CongestionWait   int64 // index 18
	SetattrTrunc     int64 // index 19
	ExtendWrite      int64 // index 20
	SillyRename      int64 // index 21
	ShortRead        int64 // index 22
	ShortWrite       int64 // index 23
	Delay            int64 // index 24
	PNFSRead         int64 // index 25
	PNFSWrite        int64 // index 26
}

// NFSMount represents a single NFS mount point
type NFSMount struct {
	Device     string
	MountPoint string
	Server     string
	Export     string
	Age        int64
	Operations map[string]*NFSOperation
	Events     *NFSEvents
	BytesRead  int64
	BytesWrite int64
}

// DeltaStats holds the difference between two measurements
type DeltaStats struct {
	Operation    string
	DeltaOps     int64
	DeltaBytes   int64
	DeltaSent    int64
	DeltaRecv    int64
	DeltaRTT     int64
	DeltaExec    int64
	DeltaQueue   int64
	DeltaErrors  int64
	DeltaRetrans int64
	AvgRTT       float64
	AvgExec      float64
	AvgQueue     float64
	KBPerOp      float64
	KBPerSec     float64
	IOPS         float64
}
