# nfs-gaze

`nfs-gaze` is a command-line tool for monitoring NFS (Network File System) client statistics on Linux. It provides real-time insights into NFS performance, similar to `nfsiostat`, but with a more user-friendly interface and additional features.

## Platform

**This tool is for Linux only.** It relies on the `/proc/self/mountstats` file, which is not available on other operating systems.

## Features

*   **Real-time Monitoring:** View NFS statistics in real-time, with a configurable update interval.
*   **Multiple Output Formats:** Choose between a simple, human-readable format and an `nfsiostat`-compatible format.
*   **Operation Filtering:** Monitor specific NFS operations (e.g., `READ`, `WRITE`) to focus on the data you need.
*   **Bandwidth and Attribute Cache Stats:** Display detailed bandwidth information and attribute cache statistics.
*   **Mount Point Selection:** Monitor a specific NFS mount point or all of them.

## Installation

1.  **Prerequisites:** You need to have Go installed on your system.
2.  **Build from source:**
    ```bash
    go build
    ```

## Usage

```bash
./nfs-gaze [options] [mount_point] [interval] [count]
```

### Options

| Flag            | Description                                       | Default                |
| --------------- | ------------------------------------------------- | ---------------------- |
| `-m`            | Mount point to monitor                            | All mounts             |
| `-ops`          | Comma-separated list of operations to monitor     | All operations         |
| `-i`            | Update interval                                   | 1s                     |
| `-c`            | Number of iterations (0 = infinite)               | 0                      |
| `-attr`         | Show attribute cache statistics                   | false                  |
| `-bw`           | Show bandwidth statistics                         | false                  |
| `-nfsiostat`    | Use nfsiostat output format                       | false                  |
| `-clear`        | Clear screen between iterations                   | false                  |
| `-f`            | Path to mountstats file                           | /proc/self/mountstats  |

### Examples

*   **Monitor in `nfsiostat` format:**
    ```bash
    ./nfs-gaze --nfsiostat /mnt/nfs --attr
    ```

*   **Monitor specific operations with bandwidth:**
    ```bash
    ./nfs-gaze -m /mnt/nfs -ops READ,WRITE -bw
    ```

*   **Clear screen between iterations:**
    ```bash
    ./nfs-gaze -m /mnt/nfs --clear
    ```

## How it Works

`nfs-gaze` works by parsing the `/proc/self/mountstats` file, which contains detailed information about NFS client activity. It reads this file at a specified interval, calculates the delta between readings, and displays the results in a user-friendly format.
