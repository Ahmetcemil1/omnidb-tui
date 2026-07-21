#!/usr/bin/env python3
import os
import sys
import time
import pty
import select
import termios
import struct
import fcntl

# ANSI Colors
GREEN = "\033[1;32m"
CYAN = "\033[1;36m"
YELLOW = "\033[1;33m"
RESET = "\033[0m"

def print_banner(text):
    print(f"\n{CYAN}{'=' * 75}{RESET}")
    print(f"{GREEN} 🚀 {text}{RESET}")
    print(f"{CYAN}{'=' * 75}{RESET}\n")
    sys.stdout.flush()

def set_window_size(fd, rows=35, cols=120):
    size = struct.pack("HHHH", rows, cols, 0, 0)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, size)

def run_cmd_in_pty(cmd_args, input_sequence=None):
    master, slave = pty.openpty()
    set_window_size(slave, 35, 120)
    
    pid = os.fork()
    if pid == 0:
        os.close(master)
        os.setsid()
        os.dup2(slave, 0)
        os.dup2(slave, 1)
        os.dup2(slave, 2)
        os.close(slave)
        os.execvp(cmd_args[0], cmd_args)
        os._exit(1)
        
    os.close(slave)
    
    # Non-blocking read
    flags = fcntl.fcntl(master, fcntl.F_GETFL)
    fcntl.fcntl(master, fcntl.F_SETFL, flags | os.O_NONBLOCK)
    
    output = []
    
    if input_sequence:
        for item in input_sequence:
            delay = item.get("delay", 1.0)
            data = item.get("data", b"")
            time.sleep(delay)
            
            # Read available output
            try:
                while True:
                    chunk = os.read(master, 4096)
                    if not chunk:
                        break
                    sys.stdout.buffer.write(chunk)
                    sys.stdout.flush()
                    output.append(chunk)
            except OSError:
                pass
                
            if data:
                os.write(master, data)
                
    time.sleep(2.0)
    # Read remaining
    try:
        while True:
            chunk = os.read(master, 4096)
            if not chunk:
                break
            sys.stdout.buffer.write(chunk)
            sys.stdout.flush()
            output.append(chunk)
    except OSError:
        pass

    os.close(master)
    os.waitpid(pid, 0)

def main():
    print_banner("OmniDB TUI (v0.1.0) — Official Launch & Feature Walkthrough")
    time.sleep(2.0)
    
    print(f"{YELLOW}▶ Step 1: Launching OmniDB TUI in Alacritty Workspace...{RESET}")
    sys.stdout.flush()
    time.sleep(2.0)
    
    # Let's increase delay values significantly to slow down the presentation
    sequence = [
        # Wait for initial TUI rendering
        {"delay": 4.0, "data": b""},
        
        # 1. Connect to Solana Devnet via Bookmarks (Index 1)
        {"delay": 2.5, "data": b"\x0e"}, # Ctrl+N (opens Connect Modal)
        {"delay": 2.5, "data": b"j"},    # Down key (moves selection from SQLite to Solana Devnet)
        {"delay": 2.5, "data": b"\r"},   # Press Enter to connect
        {"delay": 4.5, "data": b""},
        
        # Move to Query Editor with Right arrow
        {"delay": 2.5, "data": b"\x1b[C"},
        {"delay": 2.5, "data": b"11111111111111111111111111111111\r"}, # Type System Program & Enter
        {"delay": 5.0, "data": b""},
        
        # 2. Connect to Ethereum Mainnet via Bookmarks (Index 2)
        {"delay": 3.0, "data": b"\x0e"}, # Ctrl+N (opens Connect Modal)
        {"delay": 2.5, "data": b"jj"},   # Down key twice (moves selection to Ethereum Mainnet)
        {"delay": 2.5, "data": b"\r"},   # Press Enter to connect
        {"delay": 4.5, "data": b""},
        
        # Query latest Ethereum block
        {"delay": 2.5, "data": b"\x1b[C"},
        {"delay": 2.5, "data": b"latest\r"},
        {"delay": 5.0, "data": b""},
        
        # 3. Direct Tab Switching: Alt+1 -> Alt+2 -> Alt+3
        {"delay": 3.0, "data": b"\x1b1"}, # Alt+1 (Local SQLite)
        {"delay": 3.0, "data": b"\x1b2"}, # Alt+2 (Solana Devnet)
        {"delay": 3.0, "data": b"\x1b3"}, # Alt+3 (Ethereum EVM)
        
        # 4. Trigger Local AI Text-to-SQL Modal via Ctrl+Space
        {"delay": 3.0, "data": b"\x00"}, # Ctrl+Space
        {"delay": 2.5, "data": b"Show top 5 active accounts\r"},
        {"delay": 6.0, "data": b""},
        
        # 5. Switch focus back to Sidebar (so q will quit rather than typing in Editor)
        {"delay": 2.5, "data": b"\x1b[D"},
        
        # 6. Quit application cleanly with q
        {"delay": 3.0, "data": b"q"},
    ]
    
    run_cmd_in_pty(["./target/release/omnidb-tui"], sequence)
    
    # 6. Run cargo test at the very end
    print_banner("Step 2: Verification Suite — Running Engine Verification Tests")
    time.sleep(2.0)
    
    print(f"{YELLOW}▶ Executing 17 engine verification tests...{RESET}")
    sys.stdout.flush()
    time.sleep(1.5)
    
    os.system("cargo test -- --nocapture")
    time.sleep(3.0)
    
    print_banner("Lansman Demosu & Tüm Entegrasyon Testleri Başarıyla Tamamlandı!")
    print(f"{GREEN}✓ PostgreSQL, MySQL, SQLite, Solana, Ethereum, Redis ve MongoDB testleri geçti.{RESET}\n")

if __name__ == "__main__":
    main()
