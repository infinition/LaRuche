/**
 * LandDiscovery — mDNS-based node discovery for the LAND protocol.
 *
 * Listens for `_ai-inference._tcp.local.` service announcements and
 * maintains a live map of reachable LaRuche nodes.
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

export class LandDiscovery {
    private nodes = new Map<string, DiscoveredLandNode>();
    private bonjour: any = null;
    private browser: any = null;
    private active = false;

    constructor(
        private readonly onFound: NodeFoundCallback,
        private readonly onLost: NodeLostCallback,
    ) {}

    /**
     * Start mDNS browsing for LAND nodes.
     * Returns true if discovery started successfully.
     */
    start(): boolean {
        try {
            // eslint-disable-next-line @typescript-eslint/no-var-requires
            const { Bonjour } = require('bonjour-service');
            this.bonjour = new Bonjour();

            // LAND protocol service type: _ai-inference._tcp.local.
            // bonjour-service prepends _ and appends ._tcp.local. automatically.
            this.browser = this.bonjour.find({ type: 'ai-inference' });

            this.browser.on('up', (service: any) => {
                const node = this.parseService(service);
                if (!node) { return; }
                const existing = this.nodes.get(node.url);
                this.nodes.set(node.url, node);
                if (!existing) {
                    this.onFound(node);
                }
            });

            this.browser.on('down', (service: any) => {
                const host = this.extractHost(service);
                const port = (service.port as number) || 8419;
                const url = `http://${host}:${port}`;
                if (this.nodes.has(url)) {
                    this.nodes.delete(url);
                    this.onLost(url);
                }
            });

            this.active = true;
            return true;
        } catch (err) {
            console.warn('LaRuche: mDNS discovery unavailable:', err);
            return false;
        }
    }

    stop(): void {
        this.active = false;
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
        return this.nodes.get(url);
    }

    private extractHost(service: any): string {
        const addresses = service.addresses as string[] | undefined;
        if (addresses && addresses.length > 0) {
            // Prefer IPv4
            const ipv4 = addresses.find(a => !a.includes(':'));
            return ipv4 ?? addresses[0];
        }
        // Strip .local. suffix from hostname
        const host = service.host as string | undefined;
        return host ? host.replace(/\.local\.?$/, '') : 'localhost';
    }

    private parseService(service: any): DiscoveredLandNode | null {
        const host = this.extractHost(service);
        const port = (service.port as number) || 8419;
        const url = `http://${host}:${port}`;

        // TXT records are key-value pairs. bonjour-service exposes them as `service.txt`
        const txt = (service.txt as Record<string, string | undefined>) || {};

        const capabilities = Object.keys(txt)
            .filter(k => k.startsWith('capability:'))
            .map(k => k.replace('capability:', ''));

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
