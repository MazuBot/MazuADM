#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

__attribute__((noinline)) static void win(void) {
  char flag[128] = {0};

  int fd = open("/flag", O_RDONLY);
  if (fd < 0) {
    puts("flag missing");
    _exit(1);
  }

  ssize_t n = read(fd, flag, sizeof(flag) - 1);
  if (n > 0) {
    (void)write(STDOUT_FILENO, flag, (size_t)n);
  }

  close(fd);
  _exit(0);
}

__attribute__((noinline)) static void vuln(void) {
  char buf[64];

  puts("Send your payload:");
  fflush(stdout);

  (void)read(STDIN_FILENO, buf, 0x200);
  puts("bye");
}

int main(void) {
  setvbuf(stdout, NULL, _IONBF, 0);
  setvbuf(stdin, NULL, _IONBF, 0);

  char team_id[32] = {0};
  puts("TEAM ID?");
  fflush(stdout);
  if (!fgets(team_id, sizeof(team_id), stdin)) {
    return 1;
  }
  team_id[strcspn(team_id, "\n")] = '\0';
  printf("Hello, team %s!\n", team_id);

  vuln();
  return 0;
}

