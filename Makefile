
DESTDIR ?= ./out
CBINDGEN-BIN ?= cbindgen
BUILDFLAGS ?= --release
CARGO_OUT ?= target/release
SRC = $(wildcard src/*.rs)

all: ${DESTDIR}/rwm.h ${DESTDIR}/librwm.so ${DESTDIR}/librwm.a

${DESTDIR}/rwm.h: ${SRC}
	mkdir -p ${DESTDIR}
	${CBINDGEN-BIN} -l C > ${DESTDIR}/rwm.h

${DESTDIR}/librwm.%: ${SRC}
	cargo build ${BUILDFLAGS}
	mkdir -p ${DESTDIR}
	cp ${CARGO_OUT}/$(notdir $@) ${DESTDIR}/$(notdir $@)

clean:
	-rm -r ${DESTDIR}
	cargo clean
