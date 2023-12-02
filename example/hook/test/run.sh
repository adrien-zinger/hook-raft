subnet=255.255.255.248/29
img=dokken/centos-stream-9
volume=./:/servers

net=hook-net

flags="--network ${net} -dit -v ${volume}"

ip1=255.255.255.250
ip2=255.255.255.251
ip3=255.255.255.252
ip4=255.255.255.253
ip5=255.255.255.254

srv_port=8080

cmd_pre="chmod +x commit_term && \
	chmod +x switch_status && \
	chmod +x append_term && \
	chmod +x pre_append_term && \
	chmod +x retreive_term && \
	chmod +x retreive_n_term && \
	chmod +x prepare_term"

# Make settings
i=1
while test -le 5; do
	rm -f server$i/settings	> /dev/null
	cat common_settings.toml 		>> server$i/settings.toml
	cat server$i/server_settings.toml 	>> server$i/settings.toml
	i=$((i + 1))
done

timeout=10
kill_timeout=5
cmd1="cd /servers/server1 && \
	${cmd_pre} && \
	timeout ${kill_timeout} ../hook > output.log 2>&1"
cmd2="cd /servers/server2 && \
	${cmd_pre} && \
	timeout ${kill_timeout} ../hook > output.log 2>&1"
cmd3="cd /servers/server3 && \
	${cmd_pre} && \
	timeout ${timeout} ../hook > output.log 2>&1"
cmd4="cd /servers/server4 && \
	${cmd_pre} && \
	timeout ${timeout} ../hook > output.log 2>&1"
cmd5="cd /servers/server5 && \
	${cmd_pre} && \
	timeout ${timeout} ../hook > output.log 2>&1"

rm -f server*/*.log > /dev/null &2>1
rm -f server*/term_* > /dev/null &2>1
docker rm -f srv1 srv2 srv3 srv4 srv5 > /dev/null &2>1

docker network ls | grep ${net} > /dev/null
if test $? = 1; then
	docker network create --subnet $subnet ${net}
fi

docker run $flags --ip $ip1 --expose $srv_port --name srv1 $img bash
docker run $flags --ip $ip2 --expose $srv_port --name srv2 $img bash
docker run $flags --ip $ip3 --expose $srv_port --name srv3 $img bash
docker run $flags --ip $ip4 --expose $srv_port --name srv4 $img bash
docker run $flags --ip $ip5 --expose $srv_port --name srv5 $img bash

docker exec srv1 bash -c "${cmd1}" &
docker exec srv2 bash -c "${cmd2}" &
docker exec srv3 bash -c "${cmd3}" &
docker exec srv4 bash -c "${cmd4}" &
docker exec srv5 bash -c "${cmd5}" &

docker logs srv1
docker logs srv2
docker logs srv3
docker logs srv4
docker logs srv5

sleep $timeout

echo clean containers
docker rm -f srv1 srv2 srv3 srv4 srv5
