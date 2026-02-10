SHELL := /bin/bash

.PHONY: all build run clean install uninstall ccdb help

CC ?= gcc
CFLAGS ?= -Wall -Wextra -O2 -g -std=gnu99
LDFLAGS ?=

PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
BIN_DIR ?= ./.bin
TARGET ?= gshell

SRCS := $(wildcard ./src/*.c)
OBJS := $(SRCS:.c=.o)

all: build

help:
	@echo "Targets: build run install uninstall clean"

build: $(OBJS)
	@mkdir -p $(BIN_DIR)
	$(CC) $(OBJS) $(LDFLAGS) -o $(BIN_DIR)/$(TARGET)

run: build
	$(BIN_DIR)/$(TARGET)

install: build
	@install -d $(BINDIR)
	install -m 755 $(BIN_DIR)/$(TARGET) $(BINDIR)/$(TARGET)

uninstall:
	@rm -f $(BINDIR)/$(TARGET)

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f ./src/*.o

ccdb:
	bear -- make clean build
