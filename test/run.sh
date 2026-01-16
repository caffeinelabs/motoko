#!/usr/bin/env bash

# A simple test runner. Synopsis:
#
# ./run.sh foo.mo [bar.mo ..]
#
# Options:
#
#    -a: Update the files in ok/
#    -d: Run on in `drun`
#    -t: Only typecheck
#    -s: Be silent in sunny-day execution
#    -i: Only check mo to idl generation
#    -p: Produce perf statistics
#        only compiles and runs drun, writes stats to $PERF_OUT
#

function realpath() {
    [[ $1 = /* ]] && echo "$1" || echo "$PWD/${1#./}"
}


ACCEPT=no
DTESTS=no
IDL=no
PERF=no
WASMTIME_OPTIONS="-C cache=n -W nan-canonicalization=y -W memory64 -W multi-memory -W bulk-memory"
WRAP_drun=$(realpath $(dirname $0)/drun-wrapper.sh)
SKIP_RUNNING=${SKIP_RUNNING:-no}
SKIP_VALIDATE=${SKIP_VALIDATE:-no}
ONLY_TYPECHECK=no
ECHO=echo
MOC_ARGS="--legacy-persistence --legacy-actors"

export WASMTIME_NEW_CLI=1

while getopts "adpstirv" o; do
    case "${o}" in
        a)
            ACCEPT=yes
            ;;
        d)
            DTESTS=yes
            ;;
        p)
            PERF=yes
            ;;
        s)
            ECHO=true
            ;;
        t)
            ONLY_TYPECHECK=true
            ;;
        i)
            IDL=yes
            ;;
    esac
done

shift $((OPTIND-1))

failures=()

function normalize () {
  if [ -e "$1" ]
  then
    grep -a -E -v '^Raised by|^Raised at|^Re-raised at|^Re-Raised at|^Called from|^ +at ' $1 |
    grep -a -E -v 'note: using the' |
    sed -e 's/\x00//g' \
        -e 's/\x1b\[[0-9;]*[a-zA-Z]//g' \
        -e 's/^.*[IW], hypervisor:/hypervisor:/g' \
        -e 's/wasm:0x[a-f0-9]*:/wasm:0x___:/g' \
        -e 's/prelude:[^:]*:/prelude:___:/g' \
        -e 's/prim:[^:]*:/prim:___:/g' \
        -e 's/ calling func\$[0-9]*/ calling func$NNN/g' \
        -e 's/rip_addr: [0-9]*/rip_addr: XXX/g' \
        -e 's,/private/tmp/,/tmp/,g' \
        -e 's,/tmp/.*ic.[^/]*,/tmp/ic.XXX,g' \
        -e 's,/build/.*ic.[^/]*,/tmp/ic.XXX,g' \
        -e 's,^.*/idl/_out/,..../idl/_out/,g' | # node puts full paths in error messages
    sed -e 's,\([a-zA-Z0-9.-]*\).mo.mangled,\1.mo,g' \
        -e 's/trap at 0x[a-f0-9]*/trap at 0x___:/g' \
        -e 's/^\(         [0-9]\+:\).*!/\1 /g' | # wasmtime backtrace locations
    sed -e 's/^  \(         [0-9]\+:\).*!/\1 /g' | # wasmtime backtrace locations (later version)
    sed -e 's/wasm `unreachable` instruction executed/unreachable/g' | # cross-version normalisation
    sed -e 's/Ignore Diff:.*/Ignore Diff: (ignored)/ig' \
        -e 's/Motoko compiler (source .*)/Motoko compiler (source XXX)/ig' \
        -e 's/Motoko compiler [^ ]* (source .*)/Motoko compiler (source XXX)/ig' \
        -e 's/Motoko (source .*)/Motoko (source XXX)/ig' |

    # Normalize canister id prefixes and timestamps in debug prints
    sed -e 's/\[Canister [0-9a-z\-]*\]/debug.print:/g' \
        -e 's/^20.*UTC: debug.print:/debug.print:/g' \
        -e '/^ic_trap$/d' |

    # Normalize the output of the test-runner.
    # remove all lines containing PocketIC.
    sed -e '/PocketIC/d' |

    # Delete everything after Oom
    sed -e '/RTS error: Cannot grow memory/q' \
        -e '/RTS error: Cannot allocate memory/q' > $1.norm

    mv $1.norm $1
  fi
}

function run () {
  # first argument: extension of the output file
  # remaining argument: command line
  # uses from scope: $out, $file, $base, $diff_files

  local ext="$1"
  shift

  if grep -q "^//SKIP $ext$" $(basename $file); then return 1; fi
  local FILTER_LINE=$(grep -E "^//FILTER $ext [A-Za-z0-9 -]*$" $(basename $file) | cut -d' ' -f3-)
  if [[ "$FILTER_LINE" != "" ]]; then local FILTER="$FILTER_LINE"; fi

  if test -e $out/$base.$ext
  then
    echo "Output $ext already exists."
    exit 1
  fi

  $ECHO -n " [$ext]"
  $ECHO "$@" >& $out/$base.$ext
  set -o pipefail
  "$@" |& ${FILTER:-cat} > $out/$base.$ext
  local ret=$?
  set +o pipefail

  if [ $ret != 0 ]
  then echo "Return code $ret" >> $out/$base.$ext.ret
  else rm -f $out/$base.$ext.ret
  fi
  diff_files="$diff_files $base.$ext.ret"

  normalize $out/$base.$ext
  diff_files="$diff_files $base.$ext"

  return $ret
}

# 'run_stderr' is a variant of 'run' that redirects stdout and stderr into
# separate files instead of merging them. It was created by copy&paste and
# applying minor tweaks. If the 'run' function is changed, it is quite likely
# that 'run_stderr' requires similar changes.
function run_stderr () {
  # first argument: extension of the output file
  # remaining argument: command line
  # uses from scope: $out, $file, $base, $diff_files

  local ext="$1"
  shift

  if grep -q "^//SKIP $ext$" $(basename $file); then return 1; fi

  if test -e $out/$base.$ext
  then
    echo "Output $ext already exists."
    exit 1
  fi

  $ECHO -n " [$ext]"
  "$@" > $out/$base.$ext 2>$out/$base.$ext.stderr
  local ret=$?

  if [ $ret != 0 ]
  then echo "Return code $ret" >> $out/$base.$ext.ret
  else rm -f $out/$base.$ext.ret
  fi
  diff_files="$diff_files $base.$ext.ret"

  normalize $out/$base.$ext.stderr
  diff_files="$diff_files $base.$ext.stderr"

  normalize $out/$base.$ext
  diff_files="$diff_files $base.$ext"

  return $ret
}

function run_if () {
  # first argument: a file extension
  # remaining argument: passed to run

  local ext="$1"
  shift

  if test -e $out/$base.$ext
  then
    run "$@"
    # Extract instruction count for performance mode.
    if [ "$PERF" = "yes" -a -e $out/$base.drun-run ]; then
      PERF_INSTRUCTION_COUNT=$(LANG=C perl -ne "s/_//g; print \$1 if /^debug\.print: instructions: ([0-9]+)\$/" $out/$base.drun-run)
      # echo $PERF_INSTRUCTION_COUNT
      # Remove the instruction count line from the output file so that tests with different
      # performance numbers do not affect the .ok files.
      sed -i '/^debug\.print: instructions: [0-9_]*$/d' $out/$base.drun-run
    fi
  else
    return 1
  fi
}

if [ "$PERF" = "yes" ]
then
  if [ -z "$PERF_OUT" ]
  then
    echo "Warning: \$PERF_OUT not set" >&2
  fi
fi

HAVE_drun=no
HAVE_ic_wasm=no

FLAGS_drun=

if [ $DTESTS = yes -o $PERF = yes ]
then
  if test-runner --help >& /dev/null
  then
    HAVE_drun=yes
  else
    if [ $ACCEPT = yes ]
    then
      echo "ERROR: Could not run drun, cannot update expected test output"
      exit 1
    else
      echo "WARNING: Could not run drun, will skip running some tests"
      HAVE_drun=no
    fi
  fi
  # TODO: Re-enable when ic_wasm supports Wasm64 and passive data segments
  # if ic-wasm --help >& /dev/null
  # then
  #   HAVE_ic_wasm=yes
  # fi
fi


for file in "$@";
do
  if ! [ -r $file ]
  then
    echo "File $file does not exist."
    failures+=("$file")
    continue
  fi

  if [ ${file: -3} == ".mo" ]
  then base=$(basename $file .mo); ext=mo
  elif [ ${file: -3} == ".sh" ]
  then base=$(basename $file .sh); ext=sh
  elif [ ${file: -4} == ".wat" ]
  then base=$(basename $file .wat); ext=wat
  elif [ ${file: -4} == ".did" ]
  then base=$(basename $file .did); ext=did
  elif [ ${file: -4} == ".cmp" ]
  then base=$(basename $file .cmp); ext=cmp
  elif [ ${file: -5} == ".drun" ]
  then base=$(basename $file .drun); ext=drun
  else
    echo "Unknown file extension in $file"
    echo "Supported extensions: .mo .sh .wat .did .drun"
    failures+=("$file")
    continue
  fi

  # We run all commands in the directory of the .mo file,
  # so that no paths leak into the output
  pushd $(dirname $file) >/dev/null

  out=_out
  ok=ok

  $ECHO -n "$base:"
  [ -d $out ] || mkdir $out
  [ -d $ok ] || mkdir $ok

  rm -rf $out/$base $out/$base.*

  # First run all the steps, and remember what to diff
  diff_files=

  case $ext in
  "mo")
    # extra flags (allow shell variables there)
    moc_extra_flags="$(eval echo $(grep '//MOC-FLAG' $base.mo | cut -c11- | paste -sd' '))"
    moc_extra_env="$(eval echo $(grep '//MOC-ENV' $base.mo | cut -c10- | paste -sd' '))"
    if ! grep -q "//MOC-NO-FORCE-GC" $base.mo
    then
      TEST_MOC_ARGS="--force-gc $EXTRA_MOC_ARGS"
    else
      TEST_MOC_ARGS=$EXTRA_MOC_ARGS
    fi
    if grep -q "//ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY" $base.mo
    then
      if [[ $EXTRA_MOC_ARGS != *"--enhanced-orthogonal-persistence"* ]]
      then
        $ECHO " Skipped (not applicable to classical orthogonal persistence)"
        continue
      fi
    fi
    if grep -q "//CLASSICAL-PERSISTENCE-ONLY" $base.mo
    then
      if [[ $EXTRA_MOC_ARGS == *"--enhanced-orthogonal-persistence"* ]]
      then
        $ECHO " Skipped (not applicable to enhanced persistence)"
        continue
      fi
    fi
    if grep -q "//INCREMENTAL-GC-ONLY" $base.mo
    then
      if [[ $EXTRA_MOC_ARGS != *"--incremental-gc"* ]]
      then
        $ECHO " Skipped (not applicable to incremental gc)"
        continue
      fi
    fi
    if grep -q "//SKIP-SANITY-CHECKS" $base.mo
    then
      if [[ $EXTRA_MOC_ARGS == *"--sanity-checks"* ]]
      then
        $ECHO " Skipped (not applicable to --sanity-checks)"
        continue
      fi
    fi
    if [[ $moc_extra_flags == *"-measure-rts-stack"* ]]
    then
      if [[ $(uname -m) != "x86_64" ]]
      then
        $ECHO " Skipped (not applicable on experimental platforms)"
        continue
      fi
    fi
    moc_with_flags="env $moc_extra_env moc $MOC_ARGS $moc_extra_flags $TEST_MOC_ARGS"

    # Typecheck
    run tc $moc_with_flags --check $base.mo
    tc_succeeded=$?
    normalize $out/$base.tc

    if [ "$tc_succeeded" -eq 0 -a "$ONLY_TYPECHECK" = "no" ]
    then
      if [ $IDL = 'yes' ]
      then
        run idl $moc_with_flags --idl $base.mo -o $out/$base.did
        idl_succeeded=$?

        normalize $out/$base.did
        diff_files="$diff_files $base.did"

        if [ "$idl_succeeded" -eq 0 ]
        then
          run didc didc --check $out/$base.did
        fi
      else
        if [ "$SKIP_RUNNING" != yes -a "$PERF" != yes ]
        then
          # Interpret
          run run $moc_with_flags --hide-warnings -r $base.mo

          # Interpret IR without lowering
          run run-ir $moc_with_flags --hide-warnings -r -iR -no-async -no-await $base.mo

          # Diff interpretations without/with lowering
          if [ -e $out/$base.run -a -e $out/$base.run-ir ]
          then
            diff -u -N --label "$base.run" $out/$base.run --label "$base.run-ir" $out/$base.run-ir > $out/$base.diff-ir
            diff_files="$diff_files $base.diff-ir"
          fi

          # Interpret IR with lowering
          run run-low $moc_with_flags --hide-warnings -r -iR $base.mo

          # Diff interpretations without/with lowering
          if [ -e $out/$base.run -a -e $out/$base.run-low ]
          then
            diff -u -N --label "$base.run" $out/$base.run --label "$base.run-low" $out/$base.run-low > $out/$base.diff-low
            diff_files="$diff_files $base.diff-low"
          fi

        fi

        # Mangle for compilation:
        # The compilation targets do not support self-calls during canister
        # installation, so this replaces
        #
        #     actor a { … }
        #     a.go(); //OR-CALL …
        #
        # with
        #
        #     actor a { … }
        #     //CALL …
        #
        # which actually works on the IC platform

        # needs to be in the same directory to preserve relative paths :-(
        mangled=$base.mo.mangled
        sed 's,^.*//OR-CALL,//CALL,g' $base.mo > $mangled


        # Compile
        if [ $DTESTS = yes ]
        then
          run comp $moc_with_flags $FLAGS_drun --hide-warnings --map -c $mangled -o $out/$base.wasm
        elif [ $PERF = yes ]
        then
          run comp $moc_with_flags --hide-warnings --map -c $mangled -o $out/$base.wasm
          if [ $HAVE_ic_wasm = yes ]; then
            run opt ic-wasm -o $out/$base.opt.wasm $out/$base.wasm optimize O3 --keep-name-section
          fi
        else
          run comp $moc_with_flags -g -wasi-system-api --hide-warnings --map -c $mangled -o $out/$base.wasm
        fi

        if [ "$SKIP_VALIDATE" != yes ]
        then
          run_if wasm valid wasm-validate --enable-memory64 --enable-multi-memory $out/$base.wasm
        fi

        if [ -e $out/$base.wasm ]
        then
          # Check filecheck
          if [ "$SKIP_RUNNING" != yes ]
          then
            if grep -F -q ^//CHECK $mangled
            then
              $ECHO -n " [FileCheck]"
              wasm2wat --enable-memory64 --enable-multi-memory --no-check $out/$base.wasm > $out/$base.wat
              cat $out/$base.wat | FileCheck $mangled > $out/$base.filecheck 2>&1
              diff_files="$diff_files $base.filecheck"
            fi
          fi
        fi

        # Run compiled program
        if [ "$SKIP_RUNNING" != yes ]
        then
          if [ $DTESTS = yes ]
          then
            if [ $HAVE_drun = yes ]; then
              run_if wasm drun-run $WRAP_drun $out/$base.wasm $mangled
            fi
          elif [ $PERF = yes ]
          then
            if [ $HAVE_drun = yes ]; then
              run_if wasm drun-run $WRAP_drun $out/$base.wasm $mangled
              if [ -n "$PERF_OUT" ]
              then
                #LANG=C perl -ne "print \"gas/$base;\$1\n\" if /^scheduler_(?:cycles|instructions)_consumed_per_round_sum (\\d+)\$/" $out/$base.metrics >> $PERF_OUT;
                #LANG=C perl -ne "s/_//g; print \"gas/$base;\$1\n\" if /^debug\.print: instructions: ([0-9]+)\$/" $out/$base.drun-run >> $PERF_OUT;
                echo "gas/$base;$PERF_INSTRUCTION_COUNT" >> $PERF_OUT;
                unset PERF_INSTRUCTION_COUNT  # Clear for next test.
              fi
              run_if opt.wasm drun-run-opt $WRAP_drun $out/$base.opt.wasm $mangled
            fi
          else
            run_if wasm wasm-run wasmtime run $WASMTIME_OPTIONS $out/$base.wasm
          fi
        fi

        # collect size stats
        if [ "$PERF" = yes -a -e "$out/$base.wasm" ]
        then
           if [ -n "$PERF_OUT" ]
           then
             wasm-strip $out/$base.wasm -o $out/$base.wasm.strip
             echo "size/$base;$(stat --format=%s $out/$base.wasm.strip)" >> $PERF_OUT
           fi
        fi

        rm -f $mangled
      fi
    fi
  ;;
  "drun")
    if [ $DTESTS != yes ]
    then
      $ECHO ""
      echo "Running .drun files only make sense with $0 -d";
      continue
    fi

    # The file is a drun script, so a multi-canister project
    mkdir -p $out/$base

    for runner in drun
    do
      if grep -q "# *SKIP $runner" $(basename $file)
      then
        continue
      fi
      if grep -q "# ENHANCED-ORTHOGONAL-PERSISTENCE-ONLY" $(basename $file)
      then
        if [[ $EXTRA_MOC_ARGS != *"--enhanced-orthogonal-persistence"* ]]
        then
          continue
        fi
      fi
      if grep -q "# CLASSICAL-PERSISTENCE-ONLY" $(basename $file)
      then
        if [[ $EXTRA_MOC_ARGS == *"--enhanced-orthogonal-persistence"* ]]
        then
          continue
        fi
      fi
      if grep -q "# SKIP-SANITY-CHECKS" $(basename $file)
      then
        if [[ $EXTRA_MOC_ARGS == *"--sanity-checks"* ]]
        then
          continue
        fi
      fi
      if grep -q "# DEFAULT-GC-ONLY" $(basename $file)
      then
        if [[ $EXTRA_MOC_ARGS == *"--copying-gc"* ]] || [[ $EXTRA_MOC_ARGS == *"--compacting-gc"* ]] || [[ $EXTRA_MOC_ARGS == *"--generational-gc"* ]] || [[ $EXTRA_MOC_ARGS == *"--incremental-gc"* ]]
        then
          continue
        fi
      fi
      if grep -q "# INCREMENTAL-GC-ONLY" $(basename $file)
      then
        if [[ $EXTRA_MOC_ARGS != *"--incremental-gc"* ]]
        then
          $ECHO " Skipped (not applicable to incremental gc)"
          continue
        fi
      fi
      if grep -q "# APPLICATION-SUBNET" $(basename $file)
      then
        # set drun args to use application subnet
        EXTRA_DRUN_ARGS="--subnet-type application"
      fi
      
      have_var_name="HAVE_${runner//-/_}"
      if [ ${!have_var_name} != yes ]
      then
        $ECHO "skipped (no $runner)";
        continue
      fi

      # collect all .mo files referenced from the file
      mo_files="$(grep -o '[^[:space:]]\+\.mo' $base.drun |sort -u)"

      for mo_file in $mo_files
      do
        mo_base=$(basename $mo_file .mo)
        if [ "$(dirname $mo_file)" != "$base" ];
        then
          $ECHO ""
          echo "$base.drun references $mo_file which is not in directory $base"
          exit 1
        fi
        moc_extra_flags="$(eval echo $(grep '//MOC-FLAG' $mo_file | cut -c11- | paste -sd' '))"
        flags_var_name="FLAGS_${runner//-/_}"
        run $mo_base.$runner.comp moc $MOC_ARGS $EXTRA_MOC_ARGS ${!flags_var_name} $moc_extra_flags --hide-warnings -c $mo_file -o $out/$base/$mo_base.$runner.wasm
      done

      # mangle drun script
      LANG=C perl -npe "s,$base/([^\s]+)\.mo,$out/$base/\$1.$runner.wasm," < $base.drun > $out/$base/$base.$runner.drun

      # run wrapper
      wrap_var_name="WRAP_${runner//-/_}"
      run $runner ${!wrap_var_name} $out/$base/$base.$runner.drun $EXTRA_DRUN_ARGS
      # clear EXTRA_DRUN_ARGS.
      EXTRA_DRUN_ARGS=""
    done

  ;;
  "sh")
    # The file is a shell script, just run it
    $ECHO -n " [out]"
    ./$(basename $base.sh) > $out/$base.stdout 2> $out/$base.stderr
    normalize $out/$base.stdout
    normalize $out/$base.stderr
    diff_files="$diff_files $base.stdout $base.stderr"
  ;;
  "wat")
    # The file is a .wat file, so we are expected to test linking
    $ECHO -n " [mo-ld]"
    rm -f $out/$base.{base,lib,linked}.{wasm,wat,o}
    make --quiet $out/$base.{base,lib}.wasm
    mo-ld -b $out/$base.base.wasm -l $out/$base.lib.wasm -o $out/$base.linked.wasm > $out/$base.mo-ld 2>&1
    diff_files="$diff_files $base.mo-ld"

    if [ -e $out/$base.linked.wasm ]
    then
        run wasm2wat wasm2wat --enable-memory64 $out/$base.linked.wasm -o $out/$base.linked.wat
        diff_files="$diff_files $base.linked.wat"
    fi
  ;;
  "did")
    # The file is a .did file, so we are expected to test the idl
    # Typecheck
    $ECHO -n " [tc]"
    didc --check $base.did > $out/$base.tc 2>&1
    tc_succeeded=$?
    normalize $out/$base.tc
    diff_files="$diff_files $base.tc"

    if [ "$tc_succeeded" -eq 0 ];
    then
      $ECHO -n " [pp]"
      didc --pp $base.did > $out/$base.pp.did
      sed -i 's/import "/import "..\//g' $out/$base.pp.did
      didc --check $out/$base.pp.did > $out/$base.pp.tc 2>&1
      diff_files="$diff_files $base.pp.tc"

      run didc-js didc --js $base.did -o $out/$base.js
      normalize $out/$base.js
      diff_files="$diff_files $base.js"

      if [ -e $out/$base.js ]
      then
        export NODE_PATH=$NODE_PATH:$ESM
        run node node -r esm $out/$base.js
      fi
    fi
    ;;
  "cmp")
    # The file is a .cmp file, so we are expected to test compatiblity of the two
    # files in cmp
    # Compatibility check
    $ECHO -n " [cmp]"
    moc $MOC_ARGS --stable-compatible $(<$base.cmp) > $out/$base.cmp 2>&1
    succeeded=$?
    if [ "$succeeded" -eq 0 ]
    then
     echo "TRUE" >> $out/$base.cmp
    else
     echo "FALSE" >> $out/$base.cmp
    fi
    diff_files="$diff_files $base.cmp"
  ;;
  *)
    echo "Unknown extentions $ext";
    exit 1
  esac
  $ECHO ""

  if [ $ACCEPT = yes ]
  then
    rm -f $ok/$base.*

    for outfile in $diff_files
    do
      if [ -s $out/$outfile ]
      then
        cp $out/$outfile $ok/$outfile.ok
      else
        rm -f $ok/$outfile.ok
      fi
    done
  else
    for diff_file in $diff_files
    do
      if [ -e $ok/$diff_file.ok -o -e $out/$diff_file ]
      then
        diff -a -u -N --label "$diff_file (expected)" $ok/$diff_file.ok --label "$diff_file (actual)" $out/$diff_file
        if [ $? != 0 ]; then failures+=("$file");fi
      fi
    done
  fi
  popd >/dev/null
done

if [ ${#failures[@]} -gt 0  ]
then
  echo "Some tests failed:"
  tr ' ' '\n' <<< "${failures[@]}" | uniq | xargs echo
  exit 1
else
  $ECHO "All tests passed."
fi
