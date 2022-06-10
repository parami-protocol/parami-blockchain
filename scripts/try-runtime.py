from sys import argv,stderr,exit
from time import time
from os import system

def print_help_message():
    print("""
Run try-runtime on-runtime-grade:
    python3 try-runtime.py live {pallets...}
    python3 try-runtime.py snap [snap-file-name]

Example:
    python3 try-runtime.py live Ad Nft
    python3 try-runtime.py snap snap_15566633.bin
    """, file=stderr)

if __name__ == "__main__":
    if len(argv) <= 1:
        print_help_message()
        exit(1)
    if argv[1] == 'live':
        if len(argv) == 2:
            print("""
Note: You are downloading states of ALL pallets. It might take a while.
      to speed up downloading, you could specify only relavent pallet names
      Example: python3 try-runtime.py live Ad Nft
      """)
        pallet_args = "".join([f"--pallets {pallet}" for pallet in argv[2:]])
        snap_filename = f"snap_{int(time())}.bin"
        system(f'RUST_LOG=runtime=trace,try-runtime::cli=trace,executor=trace cargo run --release --bin=parami --features=try-runtime -- try-runtime --execution=Native on-runtime-upgrade live --uri "wss://rpc.parami.io:443/ws" -s {snap_filename} {pallet_args}')
        exit(0)
    if argv[1] == 'snap':
        snap_filename = argv[2]
        system(f'RUST_LOG=runtime=trace,try-runtime::cli=trace,executor=trace cargo run --release --bin=parami --features=try-runtime -- try-runtime --execution=Native on-runtime-upgrade snap -s {snap_filename}')
        exit(0)
    print_help_message()
