Vagrant.configure("2") do |config|
    config.vm.box = "ubuntu/jammy64"
  
    config.vm.network "private_network", ip: "192.168.33.10"
  
    # config.ssh.private_key_path = ['~/.vagrant.d/insecure_private_key', '.vagrant/machines/default/virtualbox/private_key', '~/.ssh/id_rsa']
    # config.ssh.forward_agent = true
  
    config.vm.provider "virtualbox" do |vb|
      # vb.gui = true
      vb.memory = 8192
      vb.cpus = 4
      vb.customize ["modifyvm", :id, "--usb", "on"]
      vb.customize ["modifyvm", :id, "--usbehci", "on"]
  
      # Add filter for MAX32660 programmer
      vb.customize ["usbfilter", "add", "0",
      "--target", :id,
      "--name", "cmsis-dap debugger",
      "--productid", "0204",
      "--vendorid", "0d28"]
      
    end
  
    config.vm.provision "shell", inline: <<-SHELL
      apt-get update
      apt-get dist-upgrade -y
      apt-get install -y build-essential libuv1-dev libudev-dev libssl-dev pkg-config linux-generic python3-pip libusb-1.0-0-dev libftdi1-dev 
      usermod -a -G dialout vagrant
      usermod -a -G plugdev vagrant
      wget https://probe.rs/files/69-probe-rs.rules
      mv 69-probe-rs.rules /etc/udev/rules.d
      udevadm control --reload
      udevadm trigger
    SHELL
  
    config.vm.provision "shell", privileged: false, inline: <<-SHELL
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
      source "$HOME/.cargo/env"
      rustup target add thumbv7em-none-eabihf
      cargo install cargo-generate
      cargo install ldproxy
      cargo install https
      cargo install cargo-make
      cargo install probe-rs --features cli
      mkdir /home/vagrant/project
      cp -r /vagrant/. /home/vagrant/project
    SHELL
  
    config.vm.provision 'shell', reboot: true
  
  end