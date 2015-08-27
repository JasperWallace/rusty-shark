initSidebarItems({"enum":[["Active","Phantom type representing an active capture handle."],["Error","An error received from pcap"],["Inactive","Phantom type representing an inactive capture handle."],["Offline","Phantom type representing an offline capture handle, from a pcap dump file. Implements `Activated` because it behaves nearly the same as a live handle."]],"struct":[["Capture","This is a pcap capture handle which is an abstraction over the `pcap_t` provided by pcap. There are many ways to instantiate and interact with a pcap handle, so phantom types are used to express these behaviors."],["Device","A network device name and (potentially) pcap's description of it."],["Linktype","This is a datalink link type."],["Packet","Represents a packet returned from pcap."],["PacketHeader","Represents a packet header provided by pcap, including the timeval, caplen and len."],["Savefile","Abstraction for writing pcap savefiles, which can be read afterwards via `Capture::from_file()`."]],"trait":[["Activated",""],["State","`Capture`s can be in different states at different times, and in these states they may or may not have particular capabilities. This trait is implemented by phantom types which allows us to punt these invariants to the type system to avoid runtime errors."]]});