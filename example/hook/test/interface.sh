#! /bin/bash

modprobe dummy

declare -a test_ips
test_ips[1]=192.168.1.1
test_ips[2]=192.168.1.2
test_ips[3]=192.168.1.3
test_ips[4]=192.168.1.4
test_ips[5]=192.168.1.5

# Create dummy interface
echo debug: Create dummy interface eth12 
ip link add eth12 type dummy
ip link show eth12

# Create a subinterface
ip link add link eth12 name eno2.10 type vlan id 10
for i in `seq 5`; do
	echo debug: Add ip addr ${test_ips[$i]}/24 dev eno2.10
	ip addr add ${test_ips[$i]}/24 dev eno2.10
done

# Restore network configuration
# ip link set eno2.10 down
# ip link delete eno2.10

# modprobe -r dummy
