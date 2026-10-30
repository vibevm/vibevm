#include <errno.h>
#include <pty.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/wait.h>
#include <unistd.h>

static int write_all(int fd, const char *buffer, size_t length) {
    while (length > 0) {
        ssize_t written = write(fd, buffer, length);
        if (written < 0 && errno == EINTR) continue;
        if (written <= 0) return -1;
        buffer += written;
        length -= (size_t)written;
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc < 2) return 120;
    int master = -1;
    int slave = -1;
    struct winsize window = { .ws_row = 24, .ws_col = 200, .ws_xpixel = 0, .ws_ypixel = 0 };
    if (openpty(&master, &slave, NULL, NULL, &window) < 0) return 121;
    pid_t child = fork();
    if (child < 0) return 122;
    if (child == 0) {
        close(master);
        if (setsid() < 0) _exit(123);
        if (ioctl(slave, TIOCSCTTY, 0) < 0) _exit(124);
        if (dup2(slave, STDIN_FILENO) < 0 ||
            dup2(slave, STDOUT_FILENO) < 0 ||
            dup2(slave, STDERR_FILENO) < 0) _exit(125);
        if (slave > STDERR_FILENO) close(slave);
        execv(argv[1], &argv[1]);
        _exit(126);
    }
    close(slave);
    const char *input = getenv("VIBE_TEST_PTY_INPUT");
    if (input != NULL && write_all(master, input, strlen(input)) < 0) return 127;
    char buffer[4096];
    for (;;) {
        ssize_t length = read(master, buffer, sizeof(buffer));
        if (length > 0) {
            if (write_all(STDOUT_FILENO, buffer, (size_t)length) < 0) return 127;
            continue;
        }
        if (length < 0 && errno == EINTR) continue;
        break;
    }
    close(master);
    int status = 0;
    while (waitpid(child, &status, 0) < 0) {
        if (errno != EINTR) return 129;
    }
    if (WIFEXITED(status)) return WEXITSTATUS(status);
    if (WIFSIGNALED(status)) return 128 + WTERMSIG(status);
    return 130;
}
