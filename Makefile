
DESTDIR=./out/
CBINDGEN-BIN=cbindgen
FILES=src/*.rs Cargo.toml

${DESTDIR}/libdwm.h: src/lib.rs Cargo.toml ${DESTDIR}/libdwm.a ${DESTDIR}/libdwm.so
	${CBINDGEN-BIN} -l C > ${DESTDIR}/libdwm.h

${DESTDIR}/libdwm.a: src/lib.rs Cargo.toml
	mkdir -p ${DESTDIR}
	cargo build --release
	cp target/release/libdwm.a ${DESTDIR}/libdwm.a

${DESTDIR}/libdwm.so: src/lib.rs Cargo.toml
	mkdir -p ${DESTDIR}
	cargo build --release
	cp target/release/libdwm.so ${DESTDIR}/libdwm.so
	-strip ${DESTDIR}/libdwm.so

clean:
	rm -r ${DESTDIR}
	rm -r target/
