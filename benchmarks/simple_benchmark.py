from crisper.client import CrisperTcpClient
from crisper.lattices import LWWPairLattice

if __name__ == "__main__":
    client = CrisperTcpClient("127.0.0.1", "127.0.0.1")
    client.put(['a'], [LWWPairLattice(0, bytes(1))])
    print(client.get(['a']))
    client.put(['b'], [LWWPairLattice(0, bytes(2))])
    print(client.get(['b']))
    client.put(['a'], [LWWPairLattice(1, bytes(10))])
    print(client.get(['a']))



