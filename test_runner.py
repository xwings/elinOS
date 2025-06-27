#!/usr/bin/env python3
"""
Automated Test Runner for elinOS
Uses pexpect to interact with QEMU and run kernel tests automatically
"""

import pexpect
import sys
import time
import argparse

class ElinOSTestRunner:
    def __init__(self, timeout=30):
        self.timeout = timeout
        self.qemu_process = None
        
    def start_qemu(self):
        """Start QEMU with elinOS"""
        print("[i] Starting elinOS in QEMU...")
        try:
            # Start QEMU with the kernel
            self.qemu_process = pexpect.spawn('make run', timeout=self.timeout)
            
            # Wait for the kernel to boot and show the prompt
            self.qemu_process.expect('elinOS>', timeout=60)  # Longer timeout for boot
            print("[o] elinOS booted successfully")
            
            return True
        except pexpect.TIMEOUT:
            print("[x] Timeout waiting for elinOS to boot")
            return False
        except pexpect.EOF:
            print("[x] QEMU process ended unexpectedly")
            return False
    
    def send_command(self, command, expected_output=None, timeout=None):
        """Send a command and optionally verify output"""
        if timeout is None:
            timeout = self.timeout
            
        print(f"[i] Sending command: {command}")
        
        try:
            # Send the command
            self.qemu_process.sendline(command)
            
            # Wait for the command to complete and return to prompt
            self.qemu_process.expect('elinOS>', timeout=timeout)
            
            # Get the output
            output = self.qemu_process.before.decode('utf-8', errors='ignore')
            print(f"[i] Output: {output.strip()}")
            
            # Check expected output if provided
            if expected_output and expected_output not in output:
                print(f"[x] Expected '{expected_output}' not found in output")
                return False
                
            return True
            
        except pexpect.TIMEOUT:
            print(f"[x] Timeout waiting for command '{command}' to complete")
            return False
        except pexpect.EOF:
            print("[x] QEMU process ended unexpectedly")
            return False
    
    def run_test_suite(self):
        """Run the complete test suite"""
        print("[i] Running elinOS Test Suite")
        print("=" * 50)
        
        tests = [
            # Basic filesystem operations
            ("ls", "Total files:"),
            ("touch aaa", "Created file"),
            ("ls", "FILE  aaa"),
            ("rm aaa", "Removed file"),
            ("touch ccc", "Created file"),
            ("mkdir aaa", "Created directory"),
            ("ls", "DIR   aaa"),
            ("rmdir aaa", "Removed directory"),
            ("rm ccc", "Removed file"),

            # File operations
            ("cat test.txt", "This is a test file for the elinOS filesystem"),  # Just check it doesn't crash
            
            # ELF execution
            ("./hello_world", "Hello World from C on elinOS!"),
            
            # System commands
            ("help", "Program Execution"),
            ("memory", "Memory Regions"),
            ("version", "elinOS"),
            ("mmap", "Total mapped"),

            # graphics
            ("graphics", "Total pixels:"),
            ("gfxtest", "ALL TESTS PASSED!"),
        ]
        
        passed = 0
        failed = 0
        
        for i, (command, expected) in enumerate(tests, 1):
            print(f"\n[{i}/{len(tests)}] Test: {command}")
            
            if self.send_command(command, expected):
                print("[o] PASS")
                passed += 1
            else:
                print("[x] FAIL")
                failed += 1
            
            # Small delay between commands
            time.sleep(1)
        
        print(f"\n[i] Test Results:")
        print(f"   Passed: {passed}")
        print(f"   Failed: {failed}")
        print(f"   Success Rate: {passed/(passed+failed)*100:.1f}%")
        
        return failed == 0
    
    def cleanup(self):
        """Clean up QEMU process"""
        if self.qemu_process and self.qemu_process.isalive():
            print("Shutting down QEMU...")
            try:
                self.qemu_process.sendline("shutdown")
                self.qemu_process.expect(pexpect.EOF, timeout=10)
            except:
                self.qemu_process.terminate()
                time.sleep(2)
                if self.qemu_process.isalive():
                    self.qemu_process.kill(9)

def main():
    parser = argparse.ArgumentParser(description='elinOS Automated Test Runner')
    parser.add_argument('--timeout', type=int, default=30,
                       help='Command timeout in seconds (default: 30)')
    
    args = parser.parse_args()
    
    runner = ElinOSTestRunner(timeout=args.timeout)
    
    try:
        # Start QEMU
        if not runner.start_qemu():
            sys.exit(1)
        
        # Run tests based on arguments
        success = runner.run_test_suite()
        
        # Exit with appropriate code
        sys.exit(0 if success else 1)
        
    except KeyboardInterrupt:
        print("\n[x] Test interrupted by user")
        sys.exit(1)
    finally:
        runner.cleanup()

if __name__ == "__main__":
    main() 