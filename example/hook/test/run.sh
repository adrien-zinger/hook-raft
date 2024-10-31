# Store current network state (eno2 required)
current_state=`ip -o -4 addr show dev eno2 | grep -oP '\d+\.\d+\.\d+\.\d+/\d+'`
ip[1]=192.168.1.1
ip[2]=192.168.1.2
ip[3]=192.168.1.3
ip[4]=192.168.1.4
ip[5]=192.168.1.5

# Create a subinterface
sudo ip link add link eno2 name eno2.10 type vlan id 10
for i in `seq 5`; do
	sudo ip addr add ${ip[$i]}/24 dev eno2.10
done
sudo ip link set eno2.10 up

# Make settings
for i in `seq 5`; do
	rm -f server$i/settings	> /dev/null
	cat common_settings.toml 		>> server$i/settings.toml
	cat server$i/server_settings.toml 	>> server$i/settings.toml

	nodes="nodes = ["
	for j in `seq 5`; do
		if test $i -ne $j; then
			nodes=$nodes\'$ip[$j]", "
		fi
	done
	nodes=$nodes"]"
	echo $nodes >> server$i/settings.toml
	echo "addr = \"${ip[$i]}\""
	
	# Also set scripts as executable
	chmod +x server$i/commit_term
	chmod +x server$i/switch_status
	chmod +x server$i/append_term
	chmod +x server$i/pre_append_term
	chmod +x server$i/retreive_term
	chmod +x server$i/retreive_n_term
	chmod +x server$i/prepare_term
done

timeout=10
kill_timeout=5

# Run servers on childs bash
bash -c "cd /servers/server1 && ${cmd_pre} && timeout ${kill_timeout} ../hook > output.log 2>&1" &
bash -c "cd /servers/server2 && ${cmd_pre} && timeout ${kill_timeout} ../hook > output.log 2>&1" &
bash -c "cd /servers/server3 && ${cmd_pre} && timeout ${kill_timeout} ../hook > output.log 2>&1" &
bash -c "cd /servers/server4 && ${cmd_pre} && timeout ${kill_timeout} ../hook > output.log 2>&1" &
bash -c "cd /servers/server5 && ${cmd_pre} && timeout ${kill_timeout} ../hook > output.log 2>&1" &

echo "Test running..."
sleep $timeout

rm -f server*/*.log > /dev/null &2>1
rm -f server*/term_* > /dev/null &2>1

# Restore network configuration
sudo ip link set eno2.10 down
sudo ip link delete eno2.10
sudo ip addr $current_state dev eno2
