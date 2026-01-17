#!/usr/bin/env python3
# Checks uniqueness of error codes, and prints out the next available
# Checks that every registered code is exercised in a test
# Checks that every code exercised in a test is registered

import re
import glob
import sys

if len(sys.argv) != 2:
    print("USAGE: python check-error-codes.py ../src/lang_utils/error_codes.ml")
    sys.exit(2)

registered_codes = set()
tested_codes = set()

# This list should only contain errors that are impossible or hard to
# exercise in our test suite (or defunct)
known_untested_codes = {
    # See issue 5050... "M0000", # internal compiler error
    "M0005", # case mismatch, hard to test on linux
    "M0020", # unresolved import, seems to be an internal error?
    "M0021", # infer forwart import type. internal, because imports are topologically sorted?
    "M0022", # imported file not loaded?
    "M0023", # failed to trigger. dead code?
    "M0025", # related to unavailable variables
    "M0040", # unknown primitive type; should be an internal error?
    "M0041", # shared function with non-async result type. how to get past syntactic check?
    "M0042", # same
    "M0053", # did not manage to trigger
    "M0054", # cannot infer type of primitive expression. Could be internal
    "M0068", # mode-specific
    "M0084", # when would the return type be inferred?
    "M0094", # hard to trigger (check_exp only applies with no type variables, but shared functions have type variables)
    "M0099", # hard to trigger (syntactic checks hit first)
    "M0100", # hard to trigger (syntactic checks hit first)
    "M0108", # mode-specific
    "M0144", # bad import, but seems to be shadowed by non-static expression
    "M0161", # Candid float32 type cannot be imported as a Motoko type
    "M0162", # Candid service constructor type not supported as Motoko type
    "M0164", # unknown record or variant label in textual representation
    "M0165", # odd expected type
    "M0181", # defunct viper error
    "M0191", # compiler warning about wasm features (hard to trigger)
    "M0232", # cannot infer type of implicit argument
    "M0240", # Migration chain broken - type mismatch 
    "M0241", # Multi-migration tuple cannot be empty 
    "M0242", # Multi-migration must be tuple 
    "M0243", # Duplicate migration
    "M0244", # Multi-migration with constructor side-effects
    "M0245", # Multi-migration field not explicitly produced/consumed
    }

def populate_error_codes():
    with open(sys.argv[1]) as fp:
        for line in fp:
            line = line.strip()
            if line.startswith("(*"):
                continue
            match = re.search(r'"(M\d+)"', line)
            if match:
                code = match.group(1)
                if code in registered_codes:
                    print("Error: Duplicate error code {}".format(code))
                    sys.exit(1)
                registered_codes.add(code)


def populate_tested_codes():
    tc_ok = glob.glob("./**/*.tc.ok", recursive=True)
    comp_ok = glob.glob("./**/*.comp.ok", recursive=True)
    cmp_ok = glob.glob("./**/*.cmp.ok", recursive=True)
    paths = tc_ok + comp_ok + cmp_ok
    for path in paths:
        with open(path) as fp:
            for line in fp:
                match = re.search(r"(?:error|warning) \[(M\d+)\]", line)
                if match:
                    code = match.group(1)
                    tested_codes.add(code)

populate_error_codes()
populate_tested_codes()

untested_codes = registered_codes - tested_codes - known_untested_codes
unregistered_codes = tested_codes - registered_codes
ignored_unregistered_codes = known_untested_codes - registered_codes
supposedly_untested_codes = tested_codes.intersection(known_untested_codes)

print("The last registered error code is {}".format(max(registered_codes)))

if untested_codes:
    print("Error: There were untested error codes:\n{}".format(untested_codes))
    print("""This likely means you should add a test exercising the given error code.
If that isn't possible or impractical add the error code to 'known_untested_codes' in 'check-error-codes.py'""")
if unregistered_codes:
    print("Error: There were unregistered codes:\n{}".format(unregistered_codes))
    print("This likely means you need to add your new error codes to 'src/lang_utils/error_codes.ml'")
if ignored_unregistered_codes:
    print("Error: There were known untested codes that weren't registered:\n{}".format(ignored_unregistered_codes))
    print("This likely means you need to remove those codes from the 'known_untested_codes' list in 'check-error-codes.py'")
if supposedly_untested_codes:
    print("Error: There were supposedly known untested codes that were tested:\n{}".format(supposedly_untested_codes))
    print("Great job! This likely means you can remove those codes from the 'known_untested_codes' list in 'check-error-codes.py'")

if untested_codes or unregistered_codes or ignored_unregistered_codes or supposedly_untested_codes:
    sys.exit(1)
