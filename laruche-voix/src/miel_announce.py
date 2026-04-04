"""Miel Protocol announcer — register this service on the local network via mDNS.

Runs zeroconf in a background thread to avoid blocking the async event loop.
"""

import socket
import uuid
import threading
from zeroconf import Zeroconf, ServiceInfo

SERVICE_TYPE = "_ai-inference._tcp.local."
PROTOCOL_VERSION = "0.2.0"


class MielAnnouncer:
    """Announce a capability on the Miel network via mDNS."""

    def __init__(
        self,
        node_name: str,
        capabilities: list[str],
        port: int,
        model: str = "",
        tier: str = "pro",
    ):
        self.node_id = str(uuid.uuid4())
        self.node_name = node_name
        self.capabilities = capabilities
        self.port = port
        self.model = model
        self.tier = tier
        self._zeroconf = None
        self._info = None
        self._thread = None

    def _get_local_ip(self) -> str:
        try:
            s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            s.connect(("8.8.8.8", 80))
            ip = s.getsockname()[0]
            s.close()
            return ip
        except Exception:
            return "127.0.0.1"

    def _do_register(self):
        """Register in a background thread (zeroconf blocks the event loop otherwise)."""
        try:
            local_ip = self._get_local_ip()

            properties = {
                "land_v": PROTOCOL_VERSION,
                "node_id": self.node_id,
                "name": self.node_name,
                "tier": self.tier,
                "port": str(self.port),
                "dash_port": "0",
                "model": self.model,
                "tps": "0.0",
                "mem_pct": "0",
                "queue": "0",
            }
            for cap in self.capabilities:
                properties[f"capability:{cap}"] = "1"

            instance_name = f"laruche-{self.node_id[:8]}"

            self._info = ServiceInfo(
                SERVICE_TYPE,
                f"{instance_name}.{SERVICE_TYPE}",
                addresses=[socket.inet_aton(local_ip)],
                port=self.port,
                properties=properties,
                server=f"{self.node_name}.local.",
            )

            self._zeroconf = Zeroconf()
            self._zeroconf.register_service(self._info)
            print(f"[Miel] Registered: {self.node_name} @ {local_ip}:{self.port} [{', '.join(self.capabilities)}]")
        except Exception as e:
            print(f"[Miel] Registration failed (non-fatal): {e}")

    def register(self):
        """Register on Miel network in a background thread."""
        self._thread = threading.Thread(target=self._do_register, daemon=True)
        self._thread.start()

    def unregister(self):
        if self._zeroconf and self._info:
            try:
                self._zeroconf.unregister_service(self._info)
                self._zeroconf.close()
            except Exception:
                pass
            print(f"[Miel] Unregistered: {self.node_name}")
