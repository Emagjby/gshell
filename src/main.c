#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main(int argc, char *argv[]) {
  setbuf(stdout, NULL);
  write(1, "$ ", 2);

  return 0;
}
