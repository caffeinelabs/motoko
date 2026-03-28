SHELL:=bash -O globstar

CLANG ?= clang-19
LLVM_LINK = llvm-link
WASM_CLANG ?= clang-19
WASM_LD ?= wasm-ld-19

#
# We manually list all the .c files of libtommath that we care about.
# (Usually the Wasm linker embedded in moc will complain if something is missing.)
#

TOMMATHFILES = \
   mp_init mp_zero mp_add mp_sub mp_mul mp_cmp \
   mp_set_u32 mp_set_i32 mp_get_i32 mp_get_mag_u32 \
   mp_set_u64 mp_set_i64 mp_get_i64 mp_get_mag_u64 \
   mp_set_double mp_get_double \
   mp_div mp_init_copy mp_neg mp_abs mp_2expt mp_expt_u32 mp_set mp_sqr \
   s_mp_add mp_cmp_mag s_mp_sub mp_grow mp_clamp \
   mp_init_size mp_exch mp_clear mp_copy mp_count_bits mp_mul_2d mp_rshd mp_mul_d mp_div_2d mp_mod_2d \
   s_mp_balance_mul s_mp_toom_mul s_mp_toom_sqr s_mp_karatsuba_sqr s_mp_sqr_fast s_mp_sqr s_mp_karatsuba_mul \
   s_mp_mul_digs_fast s_mp_mul_digs mp_init_multi mp_clear_multi mp_mul_2 mp_div_2 mp_div_3 mp_lshd mp_incr \
   mp_decr mp_add_d mp_sub_d

TOMMATHSRC ?= $(CURDIR)/../../libtommath

#
# Various libtommath flags, in particular telling it to use our own memory
# manager
#

TOMMATH_FLAGS_COMMON = \
  -DMP_MALLOC=mp_malloc \
  -DMP_REALLOC=mp_realloc \
  -DMP_CALLOC=mp_calloc \
  -DMP_FREE=mp_free \
  -DMP_MEMSET=0 \
  -DMP_FIXED_CUTOFFS \
  -DMP_PREC=4 \
  -DMP_NO_FILE \
  -D__STDC_IEC_559__

TOMMATH_FLAGS_32 = $(TOMMATH_FLAGS_COMMON) \
  -DMP_32BIT

TOMMATH_FLAGS_64 = $(TOMMATH_FLAGS_COMMON) \
  -DMP_64BIT

# Note: the above __STDC_IEC_559__ define is somewhat of a misnomer
#       as only IEEE 754 features are used.

#
# clang flags
#

CLANG_FLAGS_COMMON = \
	--compile \
   -fpic \
   -fvisibility=hidden \
   --std=c11 \
   -fno-builtin -ffreestanding \
   --optimize=s \
   -resource-dir=$(wildcard $(WASM_CLANG_LIB)/lib/clang/*)

CLANG_FLAGS_32 = $(CLANG_FLAGS_COMMON) \
   --target=wasm32

CLANG_FLAGS_64 = $(CLANG_FLAGS_COMMON) \
   --target=wasm64

#
# Build targets
#

.PHONY: all

all: mo-rts-non-incremental.wasm mo-rts-non-incremental-debug.wasm mo-rts-incremental.wasm mo-rts-incremental-debug.wasm mo-rts-eop.wasm mo-rts-eop-debug.wasm

_build:
	mkdir -p $@

_build/wasm64:
	mkdir -p $@

_build/wasm32:
	mkdir -p $@

#
# Let make automatically search the tommath directory for .c files
#

vpath %.c $(TOMMATHSRC)

#
# Building the libtommath files
#

TOMMATH_WASM_32_O=$(TOMMATHFILES:%=_build/wasm32/tommath_%.o)
TOMMATH_WASM_32_A=_build/libtommath_wasm32.a

TOMMATH_WASM_64_O=$(TOMMATHFILES:%=_build/wasm64/tommath_%.o)
TOMMATH_WASM_64_A=_build/libtommath_wasm64.a

_build/wasm32/tommath_%.o: bn_%.c | _build/wasm32
	$(WASM_CLANG) $(CLANG_FLAGS_32) $(TOMMATH_FLAGS_32) $< --output $@

_build/wasm64/tommath_%.o: bn_%.c | _build/wasm64
	$(WASM_CLANG) $(CLANG_FLAGS_64) $(TOMMATH_FLAGS_64) $< --output $@

$(TOMMATH_WASM_32_A): $(TOMMATH_WASM_32_O)
	llvm-ar rcs $@ $^
	llvm-ranlib $@

$(TOMMATH_WASM_64_A): $(TOMMATH_WASM_64_O)
	llvm-ar rcs $@ $^
	llvm-ranlib $@

#
# The rust code code of the RTS
#

# This relies on bash and globstar, see https://stackoverflow.com/questions/2483182/recursive-wildcards-in-gnu-make
RTS_RUST_FILES=$(shell ls **/*.rs)
RTS_CARGO_FILES=$(shell ls **/Cargo.toml)

RTS_COMPILE_FLAGS=-C target-feature=+bulk-memory

TOMMATH_BINDINGS_RS_32=_build/wasm32/tommath_bindings.rs
TOMMATH_BINDINGS_RS_64=_build/wasm64/tommath_bindings.rs

TOMMATH_BIND_OPTIONS = \
		--use-core \
		--no-layout-tests \
	    --allowlist-function mp_init \
	    --allowlist-function mp_init_copy \
	    --allowlist-function mp_set_u32 \
	    --allowlist-function mp_set_i32 \
	    --allowlist-function mp_get_i32 \
	    --allowlist-function mp_set_u64 \
	    --allowlist-function mp_set_i64 \
	    --allowlist-function mp_get_i64 \
	    --allowlist-function mp_set_double \
	    --allowlist-function mp_get_double \
	    --allowlist-function mp_count_bits \
	    --allowlist-function mp_cmp \
	    --allowlist-function mp_add \
	    --allowlist-function mp_sub \
	    --allowlist-function mp_mul \
	    --allowlist-function mp_div \
	    --allowlist-function mp_div_2d \
	    --allowlist-function mp_neg \
	    --allowlist-function mp_abs \
	    --allowlist-function mp_mul_2d \
	    --allowlist-function mp_expt_u32 \
	    --allowlist-function mp_2expt \
	    --allowlist-function mp_incr \
	    --blocklist-type __int32_t \
	    --blocklist-type __int64_t \
	    --blocklist-type __uint32_t \
	    --blocklist-type __uint64_t \


$(TOMMATH_BINDINGS_RS_32): | _build/wasm32
	bindgen $(TOMMATHSRC)/tommath.h \
	    -o $@ \
	    --ctypes-prefix="crate::libc_declarations" \
		$(TOMMATH_BIND_OPTIONS) \
		-- $(TOMMATH_FLAGS_32)

$(TOMMATH_BINDINGS_RS_64): | _build/wasm64
	bindgen $(TOMMATHSRC)/tommath.h \
	    -o $@ \
	    --ctypes-prefix="crate::libc_declarations" \
		$(TOMMATH_BIND_OPTIONS) \
		-- $(TOMMATH_FLAGS_64)

	# Whitelist parameters used as libtommath.h has lots of definitions that we don't
	# need. Blacklist parameters are used because bindgen still generates unused type
	# definition with the whitelist parameters.
	#
	# Note that bindgen can't generate Rust macros or functions for CPP macros, so
	# macros like `mp_get_u32` and `mp_isneg` need to be manually implemented.

# 32-bit Wasm builds

RTS_DEPENDENCIES_32=$(TOMMATH_BINDINGS_RS_32) $(RTS_RUST_FILES) $(RTS_CARGO_FILES) | _build/wasm32
RTS_BUILD_32=cd motoko-rts && RUSTFLAGS="${RTS_COMPILE_FLAGS}" cargo build --target wasm32-none-shared.json -Zbuild-std=core,alloc
RTS_DEBUG_BUILD_32=$(RTS_BUILD_32)
RTS_RELEASE_BUILD_32=$(RTS_BUILD_32) --release -Zbuild-std-features="panic_immediate_abort,optimize_for_size"
RTS_DEBUG_TARGET_32=motoko-rts/target/wasm32-none-shared/debug/libmotoko_rts.a
RTS_RELEASE_TARGET_32=motoko-rts/target/wasm32-none-shared/release/libmotoko_rts.a

RTS_RUST_NON_INCREMENTAL_WASM_32_A=_build/wasm32/libmotoko_rts.a
RTS_RUST_NON_INCREMENTAL_DEBUG_WASM_32_A=_build/wasm32/libmotoko_rts_debug.a

RTS_RUST_INCREMENTAL_WASM_32_A=_build/wasm32/libmotoko_rts_incremental.a
RTS_RUST_INCREMENTAL_DEBUG_WASM_32_A=_build/wasm32/libmotoko_rts_incremental_debug.a

$(RTS_RUST_NON_INCREMENTAL_WASM_32_A): $(RTS_DEPENDENCIES_32)
	$(RTS_RELEASE_BUILD_32) --features classical_persistence
	cp $(RTS_RELEASE_TARGET_32) $@

$(RTS_RUST_NON_INCREMENTAL_DEBUG_WASM_32_A): $(RTS_DEPENDENCIES_32)
	$(RTS_DEBUG_BUILD_32) --features classical_persistence
	cp $(RTS_DEBUG_TARGET_32) $@

$(RTS_RUST_INCREMENTAL_WASM_32_A): $(RTS_DEPENDENCIES_32)
	$(RTS_RELEASE_BUILD_32) --features classical_persistence,incremental_gc
	cp $(RTS_RELEASE_TARGET_32) $@

$(RTS_RUST_INCREMENTAL_DEBUG_WASM_32_A): $(RTS_DEPENDENCIES_32)
	$(RTS_DEBUG_BUILD_32) --features classical_persistence,incremental_gc
	cp $(RTS_DEBUG_TARGET_32) $@

# 64-bit Wasm builds

RTS_DEPENDENCIES_64=$(TOMMATH_BINDINGS_RS_64) $(RTS_RUST_FILES) $(RTS_CARGO_FILES) | _build/wasm64
RTS_BUILD_64=cd motoko-rts && RUSTFLAGS="${RTS_COMPILE_FLAGS}" cargo build --target wasm64-none-shared.json -Zbuild-std=core,alloc --features enhanced_orthogonal_persistence
RTS_DEBUG_BUILD_64=$(RTS_BUILD_64)
RTS_RELEASE_BUILD_64=$(RTS_BUILD_64) --release -Zbuild-std-features="panic_immediate_abort,optimize_for_size"

RTS_DEBUG_TARGET_64=motoko-rts/target/wasm64-none-shared/debug/libmotoko_rts.a
RTS_RELEASE_TARGET_64=motoko-rts/target/wasm64-none-shared/release/libmotoko_rts.a

RTS_RUST_WASM_64_A=_build/wasm64/libmotoko_rts.a
RTS_RUST_DEBUG_WASM_64_A=_build/wasm64/libmotoko_rts_debug.a

$(RTS_RUST_WASM_64_A): $(RTS_DEPENDENCIES_64)
	$(RTS_RELEASE_BUILD_64)
	cp $(RTS_RELEASE_TARGET_64) $@

$(RTS_RUST_DEBUG_WASM_64_A): $(RTS_DEPENDENCIES_64)
	$(RTS_DEBUG_BUILD_64)
	cp $(RTS_DEBUG_TARGET_64) $@

#
# The test suite
#

TEST_DEPENDENCIES_32=$(TOMMATH_WASM_32_A) $(TOMMATH_BINDINGS_RS_32)
TEST_BUILD_32=cd motoko-rts-tests && RUSTFLAGS="${RTS_COMPILE_FLAGS}" cargo build --target=wasm32-wasip1
TEST_RUN_32=wasmtime -C cache=n -W nan-canonicalization=y motoko-rts-tests/target/wasm32-wasip1/debug/motoko-rts-tests.wasm

TEST_DEPENDENCIES_64=$(TOMMATH_WASM_64_A) $(TOMMATH_BINDINGS_RS_64)
TEST_BUILD_64=cd motoko-rts-tests && RUSTFLAGS="${RTS_COMPILE_FLAGS}" cargo build --target=wasm64-unknown-unknown -Zbuild-std=core,alloc,std,panic_abort --features enhanced_orthogonal_persistence
TEST_RUN_64=wasmtime -C cache=n -W nan-canonicalization=y -W memory64 motoko-rts-tests/target/wasm64-unknown-unknown/debug/motoko-rts-tests.wasm

.PHONY: test

test: test32-non-incremental test32-incremental test64

test32-non-incremental: $(TEST_DEPENDENCIES_32)
	$(TEST_BUILD_32) --features classical_persistence
	$(TEST_RUN_32)

test32-incremental: $(TEST_DEPENDENCIES_32)
	$(TEST_BUILD_32) --features classical_persistence,incremental_gc
	$(TEST_RUN_32)

test64: $(TEST_DEPENDENCIES_64)
	$(TEST_BUILD_64)
	$(TEST_RUN_64)

#
# Putting it all together
#

EXPORTED_SYMBOLS=\
  __wasm_call_ctors \
  __wasm_apply_data_relocs \
  memcmp \
  tan \
  asin \
  acos \
  atan \
  atan2 \
  pow \
  sin \
  cos \
  exp \
  fmod \
  fmodf \
  log

WASM_A_DEPENDENCIES_32=$(TOMMATH_WASM_32_A)
LINKER_OPTIONS_32=\
  --import-memory --shared --no-entry --gc-sections \
  $(EXPORTED_SYMBOLS:%=--export=%) --import-undefined \
  --whole-archive --experimental-pic

WASM_A_DEPENDENCIES_64=$(TOMMATH_WASM_64_A)
LINKER_OPTIONS_64=\
  -mwasm64 --import-memory --shared --no-entry --gc-sections \
  $(EXPORTED_SYMBOLS:%=--export=%) --import-undefined \
  --whole-archive --experimental-pic

mo-rts-non-incremental.wasm: $(RTS_RUST_NON_INCREMENTAL_WASM_32_A) $(WASM_A_DEPENDENCIES_32)
	$(WASM_LD) -o $@ \
	  $(LINKER_OPTIONS_32) \
	  $+
	wasm-validate $@

mo-rts-non-incremental-debug.wasm: $(RTS_RUST_NON_INCREMENTAL_DEBUG_WASM_32_A) $(WASM_A_DEPENDENCIES_32)
	$(WASM_LD) -o $@ \
	  $(LINKER_OPTIONS_32) \
	  $+
	wasm-validate $@

mo-rts-incremental.wasm: $(RTS_RUST_INCREMENTAL_WASM_32_A) $(WASM_A_DEPENDENCIES_32)
	$(WASM_LD) -o $@ \
	  $(LINKER_OPTIONS_32) \
	  $+
	wasm-validate $@

mo-rts-incremental-debug.wasm: $(RTS_RUST_INCREMENTAL_DEBUG_WASM_32_A) $(WASM_A_DEPENDENCIES_32)
	$(WASM_LD) -o $@ \
	  $(LINKER_OPTIONS_32) \
	  $+
	wasm-validate $@

mo-rts-eop.wasm: $(RTS_RUST_WASM_64_A) $(WASM_A_DEPENDENCIES_64)
	$(WASM_LD) -o $@ \
	  $(LINKER_OPTIONS_64) \
	  $+
	wasm-validate --enable-memory64 $@

mo-rts-eop-debug.wasm: $(RTS_RUST_DEBUG_WASM_64_A) $(WASM_A_DEPENDENCIES_64)
	$(WASM_LD) -o $@ \
	  $(LINKER_OPTIONS_64) \
	  $+
	wasm-validate --enable-memory64 $@

format:
	cargo fmt --verbose --manifest-path motoko-rts/Cargo.toml
	cargo fmt --verbose --manifest-path motoko-rts-tests/Cargo.toml
	cargo fmt --verbose --manifest-path motoko-rts-macros/Cargo.toml

clean:
	rm -rf \
	  _build \
	  mo-rts*.wasm \
	  motoko-rts/target \
	  motoko-rts-tests/target \
	  motoko-rts-macros/target \
	  motoko-rts/cargo-home \
