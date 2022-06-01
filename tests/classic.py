import subprocess
import os
import signal
import toml
# todo, generate the configurations

# run the first node
node_base = {
    "addr": "0.0.0.0",
    "port": "123456",
    "follower": False,
}
with open("node_base.toml", "w") as file:
    file.write(toml.dumps(node_base))
    file.close()

stream = subprocess.Popen('../target/debug/hook node_base.toml', shell=True)
#stream.send_signal(signal.SIGINT)