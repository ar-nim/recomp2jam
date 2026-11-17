// rig-relay-fastfail.c — run-scoped LD_PRELOAD interposer for the 7.55 exe's
// dead relay connect (issues #321/#323/#324).
//
// Evidence pins (OTwo.exe 7.55, 5,029,888 B, sha256 83a04541...72c823b):
//   - relay address string "203.238.136.52" at file offset 0x24e368
//   - connect sites 0x1936e1 / 0x1936fb in method 0x593650; the method also
//     writes the port literal 0x1e6c (7788) at 0x59370b
//   - without this interposer the synchronous connect blocks the UI thread
//     35.785 s until STATUS_IO_TIMEOUT (b6-winsock-room/client.log:212-219,
//     252-254) and the no-response modal fires on the queue walk after
//     unblock; with it the connect fails in 0 ms and the row paints in
//     under 2 s (b8-relay-fastfail/client.log:218-221, wire:18,21)
//
// The interposer fails ONLY that one address/port pair and passes every
// other connect() through untouched. It is loaded exclusively through
// LD_PRELOAD in the Wine launch environment (scripts/rig-wine.sh launch),
// so it lives and dies with the run: no firewall, no routing, no root.
// stderr carries the interception line; RELAY_FASTFAIL_LOG, when set, gets
// an append copy for the pack.
#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

static const unsigned char relay_address_bytes[4] = {203, 238, 136, 52};
static const unsigned char relay_port_bytes[2] = {0x1e, 0x6c};

static int (*resolve_connect(void))(int, const struct sockaddr *, socklen_t)
{
    static int (*real_connect)(int, const struct sockaddr *, socklen_t);
    if (real_connect == NULL)
        real_connect = dlsym(RTLD_NEXT, "connect");
    return real_connect;
}

static int targets_dead_relay(const struct sockaddr *address, socklen_t length)
{
    if (address == NULL || length < sizeof(struct sockaddr_in) ||
        address->sa_family != AF_INET)
        return 0;
    const struct sockaddr_in *internet = (const struct sockaddr_in *)address;
    return memcmp(&internet->sin_addr, relay_address_bytes, 4) == 0 &&
           memcmp(&internet->sin_port, relay_port_bytes, 2) == 0;
}

static void write_whole(int descriptor, const char *line)
{
    (void)write(descriptor, line, strlen(line));
}

static void log_interception(int descriptor)
{
    struct timespec now;
    clock_gettime(CLOCK_REALTIME, &now);
    char line[256];
    snprintf(line, sizeof(line),
             "[relay-fastfail] %lld.%03ld pid=%d fd=%d connect "
             "203.238.136.52:7788 -> ECONNREFUSED\n",
             (long long)now.tv_sec, now.tv_nsec / 1000000L,
             (int)getpid(), descriptor);
    write_whole(STDERR_FILENO, line);
    const char *path = getenv("RELAY_FASTFAIL_LOG");
    if (path != NULL && *path != '\0') {
        int file = open(path, O_WRONLY | O_CREAT | O_APPEND, 0644);
        if (file >= 0) {
            write_whole(file, line);
            close(file);
        }
    }
}

int connect(int descriptor, const struct sockaddr *address, socklen_t length)
{
    if (targets_dead_relay(address, length)) {
        log_interception(descriptor);
        errno = ECONNREFUSED;
        return -1;
    }
    int (*real_connect)(int, const struct sockaddr *, socklen_t) =
        resolve_connect();
    if (real_connect == NULL) {
        errno = ENOSYS;
        return -1;
    }
    return real_connect(descriptor, address, length);
}
