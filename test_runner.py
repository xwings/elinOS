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
        print("🚀 Starting elinOS in QEMU...")
        try:
            # Start QEMU with the kernel
            self.qemu_process = pexpect.spawn('make run', timeout=self.timeout)
            
            # Wait for the kernel to boot and show the prompt
            self.qemu_process.expect('elinOS>', timeout=60)  # Longer timeout for boot
            print("✅ elinOS booted successfully")
            
            return True
        except pexpect.TIMEOUT:
            print("❌ Timeout waiting for elinOS to boot")
            return False
        except pexpect.EOF:
            print("❌ QEMU process ended unexpectedly")
            return False
    
    def send_command(self, command, expected_output=None, timeout=None):
        """Send a command and optionally verify output"""
        if timeout is None:
            timeout = self.timeout
            
        print(f"📝 Sending command: {command}")
        
        try:
            # Send the command
            self.qemu_process.sendline(command)
            
            # Wait for the command to complete and return to prompt
            self.qemu_process.expect('elinOS>', timeout=timeout)
            
            # Get the output
            output = self.qemu_process.before.decode('utf-8', errors='ignore')
            print(f"📄 Output: {output.strip()}")
            
            # Check expected output if provided
            if expected_output and expected_output not in output:
                print(f"❌ Expected '{expected_output}' not found in output")
                return False
                
            return True
            
        except pexpect.TIMEOUT:
            print(f"❌ Timeout waiting for command '{command}' to complete")
            return False
        except pexpect.EOF:
            print("❌ QEMU process ended unexpectedly")
            return False
    
    def run_test_suite(self):
        """Run the complete test suite"""
        print("🧪 Running elinOS Test Suite")
        print("=" * 50)
        
        tests = [
            # Basic filesystem operations
            ("ls", "Total files:"),
            ("touch aaa", "Created file"),
            ("ls", "aaa"),
            ("rm aaa", "Removed file"),
            ("touch ccc", "Created file"),
            ("mkdir aaa", "Created directory"),
            ("ls", "aaa"),
            ("rmdir aaa", "Removed directory"),
            ("rm ccc", "Removed file"),

            # File operations
            ("cat test.txt", "This"),  # Just check it doesn't crash
            
            # ELF execution
            ("./hello_world", "Hello"),
            
            # System commands
            ("help", "Program Execution"),
            ("memory", "Memory Regions"),
            ("version", "elinOS"),
        ]
        
        passed = 0
        failed = 0
        
        for i, (command, expected) in enumerate(tests, 1):
            print(f"\n[{i}/{len(tests)}] Test: {command}")
            
            if self.send_command(command, expected):
                print("✅ PASS")
                passed += 1
            else:
                print("❌ FAIL")
                failed += 1
            
            # Small delay between commands
            time.sleep(1)
        
        print(f"\n🎯 Test Results:")
        print(f"   Passed: {passed}")
        print(f"   Failed: {failed}")
        print(f"   Success Rate: {passed/(passed+failed)*100:.1f}%")
        
        return failed == 0
    
    def run_builtin_tests(self):
        """Run the built-in kernel test suite"""
        print("🧪 Running Built-in Test Suite")
        print("=" * 50)
        
        # Run the built-in test command
        if self.send_command("test", "Test Summary", timeout=60):
            print("✅ Built-in tests completed")
            return True
        else:
            print("❌ Built-in tests failed")
            return False
    
    def cleanup(self):
        """Clean up QEMU process"""
        if self.qemu_process and self.qemu_process.isalive():
            print("🛑 Shutting down QEMU...")
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
    parser.add_argument('--builtin', action='store_true', 
                       help='Run built-in kernel tests only')
    parser.add_argument('--quick', action='store_true',
                       help='Run quick test suite')
    parser.add_argument('--timeout', type=int, default=30,
                       help='Command timeout in seconds (default: 30)')
    
    args = parser.parse_args()
    
    runner = ElinOSTestRunner(timeout=args.timeout)
    
    try:
        # Start QEMU
        if not runner.start_qemu():
            sys.exit(1)
        
        # Run tests based on arguments
        if args.builtin:
            success = runner.run_builtin_tests()
        elif args.quick:
            success = runner.send_command("test quick", "Test Summary", timeout=60)
        else:
            success = runner.run_test_suite()
        
        # Exit with appropriate code
        sys.exit(0 if success else 1)
        
    except KeyboardInterrupt:
        print("\n🛑 Test interrupted by user")
        sys.exit(1)
    finally:
        runner.cleanup()

if __name__ == "__main__":
    main() 