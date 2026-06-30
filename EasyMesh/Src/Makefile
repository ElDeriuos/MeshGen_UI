TARGET = Easy
LIBS = -lm
CC = g++
CFLAGS = -g -Wall

# Detect Operating System
ifeq ($(OS),Windows_NT)
    RM = del /f /q
    EXE = .exe
else
    RM = rm -f
    EXE =
endif

.PHONY: default all clean

default: $(TARGET)$(EXE)
all: default

OBJECTS = $(patsubst %.cpp, %.o, $(wildcard *.cpp))
HEADERS = $(wildcard *.h)

%.o: %.c $(HEADERS)
	$(CC) $(CFLAGS) -c $< -o $@

.PRECIOUS: $(TARGET)$(EXE) $(OBJECTS)

$(TARGET)$(EXE): $(OBJECTS)
	$(CC) $(OBJECTS) -Wall $(LIBS) -o $@

clean:
	-$(RM) *.o
	-$(RM) $(TARGET)$(EXE)
