/**
 * LandDiscovery - mDNS-based node discovery for the LAND protocol.
 *
 * Listens for `_ai-inference._tcp.local.` service announcements and
 * maintains a live map of reachable LaRuche nodes.
 *
 * Key fixes for cross-platform and Rust `mdns-sd` interoperability:
 * - Explicit reuseAddr for socket sharing with the Rust daemon
 * - Periodic re-browse to catch late/missed announcements
 * - Binds to 0.0.0.0 to receive multicast on all interfaces
 * - Debug logging for easier troubleshooting
 *
 * Falls back gracefully if bonjour-service is unavailable (e.g. in
 * restricted network environments) and lets the user configure a node
 * URL manually.
 */

export interface DiscoveredLandNode {
    name: string;
    host: string;
    port: number;
    url: string;
    capabilities: string[];
    model?: string;
    tokensPerSec?: number;
    tier?: string;
}

export type NodeFoundCallback = (node: DiscoveredLandNode) => void;
export type NodeLostCallback = (url: string) => void;

/** LAND protocol service type */
const LAND_SERVICE_TYPE = 'ai-inference';
/** Re-browse interval to catch missed mDNS announcements (ms) */
const REBROWSE_INTERVAL_MS = 15000;
/** Grace period before emitting node lost (absorbs transient mDNS DOWN events) */
const NODE_LOST_GRACE_MS = 12000;

export class LandDiscovery {
    // Keyed by service endpoint (IPv4 when available, otherwise host:port fallback).
    private nodes = new Map<string, DiscoveredLandNode>();
    private pendingNodeLost = new Map<string, ReturnType<typeof setTimeout>>();
    private bonjour: any = null;
    private browser: any = null;
    private active = false;
    private rebrowseTimer: ReturnType<typeof setInterval> | null = null;

    constructor(
        private readonly onFound: NodeFoundCallback,
        private readonly onLost: NodeLostCallback,
    ) { }

    /**
     * Start mDNS browsing for LAND nodes.
     * Returns true if discovery started successfully.
     */
    start(): boolean {
        try {
            // eslint-disable-next-line @typescript-eslint/no-var-requires
            const { Bonjour } = require('bonjour-service');

            // Pass options to the underlying multicast-dns instance:
            // - reuseAddr: allows coexistence with Rust mdns-sd daemon on same machine
            // - loopback: receive our own packets (needed for localhost detection)
            // - bind to 0.0.0.0 to receive multicast from ALL network interfaces
            this.bonjour = new Bonjour({
                reuseAddr: true,
                loopback: true,
                interface: '0.0.0.0',
            });

            this.startBrowsing();

            // Periodic re-browse: some mDNS implementations (especially Rust mdns-sd)
            // send announcements that may be missed by the initial query.
            // Re-creating the browser forces fresh PTR queries on the network.
            this.rebrowseTimer = setInterval(() => {
                console.log('LaRuche: Periodic LAND re-browse...');
                this.restartBrowsing();
            }, REBROWSE_INTERVAL_MS);

            this.active = true;
            console.log('LaRuche: LAND mDNS discovery started (type: _ai-inference._tcp)');
            return true;
        } catch (err) {
            console.warn('LaRuche: mDNS discovery unavailable:', err);
            return false;
        }
    }

    /**
     * Start or restart the mDNS browser for LAND services.
     */
    private startBrowsing(): void {
        // LAND protocol service type: _ai-inference._tcp.local.
        // bonjour-service prepends _ and appends ._tcp.local. automatically.
        this.browser = this.bonjour.find({ type: LAND_SERVICE_TYPE });

        this.browser.on('up', (service: any) => {
            console.log('LaRuche: mDNS service UP:', JSON.stringify({
                name: service.name,
                host: service.host,
                port: service.port,
                addresses: service.addresses,
                txt: service.txt,
            }));

            const ip = this.extractPrimaryIPv4(service);
            const node = this.parseService(service, ip);
            if (!node) {
                console.warn('LaRuche: Could not parse mDNS service:', service.name);
                return;
            }

            const key = this.serviceKey(service, node, ip);
            this.clearPendingRemoval(key);
            const existing = this.nodes.get(key);
            this.nodes.set(key, node);

            // Same service key rediscovered with a different endpoint: replace old entry.
            if (existing && existing.url !== node.url) {
                console.log(`LaRuche: LAND node replaced (${key}): ${existing.url} -> ${node.url}`);
                this.onLost(existing.url);
                this.onFound(node);
                return;
            }

            if (!existing) {
                console.log(`LaRuche: LAND node discovered: ${node.name} @ ${node.url} (key: ${key})`);
                this.onFound(node);
                return;
            }

            if (!this.sameNode(existing, node)) {
                console.log(`LaRuche: LAND node updated (${key}): ${node.name} @ ${node.url}`);
                this.onFound(node);
            }
        });

        this.browser.on('down', (service: any) => {
            const key = this.serviceKey(service);
            const existing = this.nodes.get(key);
            if (existing) {
                console.log(`LaRuche: mDNS service DOWN (${key}) -> grace ${NODE_LOST_GRACE_MS}ms`);
                this.scheduleNodeRemoval(key, existing.url);
                return;
            }

            // Fallback when DOWN event uses a slightly different key shape.
            const host = this.extractHost(service);
            const port = (service.port as number) || 8419;
            const url = this.formatHttpUrl(host, port);
            const entry = this.findByUrl(url);
            if (entry) {
                console.log(`LaRuche: mDNS service DOWN (fallback ${entry.key}) -> grace ${NODE_LOST_GRACE_MS}ms`);
                this.scheduleNodeRemoval(entry.key, entry.node.url);
            }
        });
    }

    /**
     * Restart the browser to force new PTR queries.
     * This helps discover nodes that were announced before the browser started
     * or whose announcements were missed.
     */
    private restartBrowsing(): void {
        try {
            if (this.browser) {
                this.browser.stop();
                this.browser = null;
            }
            this.startBrowsing();
        } catch (err) {
            console.warn('LaRuche: Error restarting mDNS browser:', err);
        }
    }

    stop(): void {
        this.active = false;
        if (this.rebrowseTimer) {
            clearInterval(this.rebrowseTimer);
            this.rebrowseTimer = null;
        }
        for (const timer of this.pendingNodeLost.values()) {
            clearTimeout(timer);
        }
        this.pendingNodeLost.clear();
        try {
            if (this.browser) { this.browser.stop(); this.browser = null; }
            if (this.bonjour) { this.bonjour.destroy(); this.bonjour = null; }
        } catch { /* ignore shutdown errors */ }
    }

    isActive(): boolean {
        return this.active;
    }

    getNodes(): DiscoveredLandNode[] {
        return Array.from(this.nodes.values());
    }

    getNode(url: string): DiscoveredLandNode | undefined {
        return Array.from(this.nodes.values()).find(node => node.url === url);
    }

    private findByUrl(url: string): { key: string; node: DiscoveredLandNode } | undefined {
        for (const [key, node] of this.nodes.entries()) {
            if (node.url === url) {
                return { key, node };
            }
        }
        return undefined;
    }

    private sameNode(a: DiscoveredLandNode, b: DiscoveredLandNode): boolean {
        if (a.name !== b.name) { return false; }
        if (a.host !== b.host) { return false; }
        if (a.port !== b.port) { return false; }
        if (a.url !== b.url) { return false; }
        if (a.model !== b.model) { return false; }
        if (a.tokensPerSec !== b.tokensPerSec) { return false; }
        if (a.tier !== b.tier) { return false; }
        if (a.capabilities.length !== b.capabilities.length) { return false; }
        return a.capabilities.every((cap, i) => cap === b.capabilities[i]);
    }

    private extractPrimaryIPv4(service: any): string | undefined {
        const addresses = service.addresses as string[] | undefined;
        if (!addresses || addresses.length === 0) {
            return undefined;
        }
        return addresses.find((a: string) => /^\d+\.\d+\.\d+\.\d+$/.test(a));
    }

    private extractHost(service: any): string {
        const addresses = service.addresses as string[] | undefined;
        if (addresses && addresses.length > 0) {
            // Prefer IPv4 addresses (filter out IPv6 link-local etc.)
            const ipv4 = addresses.find((a: string) => /^\d+\.\d+\.\d+\.\d+$/.test(a));
            if (ipv4) { return ipv4; }
            // Fallback to first non-link-local address
            const nonLinkLocal = addresses.find((a: string) => !a.startsWith('fe80'));
            return (nonLinkLocal ?? addresses[0]).split('%')[0];
        }
        // Strip .local. suffix from hostname
        const host = service.host as string | undefined;
        return host ? host.replace(/\.local\.?$/, '') : 'localhost';
    }

    private formatHttpUrl(host: string, port: number): string {
        const needsBracket = host.includes(':') && !host.startsWith('[') && !host.endsWith(']');
        const safeHost = needsBracket ? `[${host}]` : host;
        return `http://${safeHost}:${port}`;
    }

    private serviceKey(service: any, node?: DiscoveredLandNode, ipv4?: string): string {
        if (ipv4 && node) {
            return `${ipv4}:${node.port}`;
        }
        if (ipv4) {
            const port = (service.port as number) || 8419;
            return `${ipv4}:${port}`;
        }
        if (node) {
            return `${node.host}:${node.port}`.toLowerCase();
        }
        const host = this.extractHost(service).toLowerCase();
        const port = (service.port as number) || 8419;
        return `${host}:${port}`;
    }

    private clearPendingRemoval(key: string): void {
        const timer = this.pendingNodeLost.get(key);
        if (timer) {
            clearTimeout(timer);
            this.pendingNodeLost.delete(key);
        }
    }

    private scheduleNodeRemoval(key: string, url: string): void {
        this.clearPendingRemoval(key);
        const timer = setTimeout(() => {
            this.pendingNodeLost.delete(key);
            const existing = this.nodes.get(key);
            if (!existing || existing.url !== url) {
                return;
            }
            this.nodes.delete(key);
            this.onLost(url);
        }, NODE_LOST_GRACE_MS);
        this.pendingNodeLost.set(key, timer);
    }

    private parseService(service: any, hostOverride?: string): DiscoveredLandNode | null {
        const host = hostOverride ?? this.extractHost(service);
        const port = (service.port as number) || 8419;
        const url = this.formatHttpUrl(host, port);

        // TXT records are key-value pairs. bonjour-service exposes them as `service.txt`
        const txt = (service.txt as Record<string, string | undefined>) || {};

        // LAND protocol TXT capabilities use the format "capability:llm"
        // The Rust node broadcasts them as top-level TXT keys
        const capabilities: string[] = [];

        // Method 1: Look for "capability:X" keys (LAND protocol format from Rust mdns-sd)
        for (const key of Object.keys(txt)) {
            if (key.startsWith('capability:')) {
                capabilities.push(key.replace('capability:', ''));
            }
        }

        // Method 2: Some mDNS libraries flatten TXT differently -
        // also check for bare capability flags like "llm", "code", "vlm" etc.
        const knownCaps = ['llm', 'vlm', 'vla', 'rag', 'audio', 'image', 'embed', 'code'];
        for (const cap of knownCaps) {
            if (txt[cap] === '1' && !capabilities.includes(cap)) {
                capabilities.push(cap);
            }
        }

        capabilities.sort();

        const tpsRaw = txt['tps'];
        const tokensPerSec = tpsRaw ? parseFloat(tpsRaw) : undefined;

        return {
            name: (service.name as string) || (txt['name'] as string) || host,
            host,
            port,
            url,
            capabilities,
            model: txt['model'] || undefined,
            tokensPerSec: isNaN(tokensPerSec ?? NaN) ? undefined : tokensPerSec,
            tier: txt['tier'] || undefined,
        };
    }
}
