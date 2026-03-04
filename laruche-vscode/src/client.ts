import * as http from 'http';
import * as https from 'https';

export interface SwarmData {
    swarm_id: string;
    total_nodes: number;
    collective_tps: number;
    collective_queue: number;
    total_vram_mb: number;
    total_ram_mb: number;
    nodes: NodeInfo[];
}

export interface NodeInfo {
    node_id: string | null;
    name: string | null;
    host: string;
    capabilities: string[];
    tokens_per_sec: number | null;
    queue_depth: number | null;
}

export interface InferResponse {
    response: string;
    model: string;
    tokens_generated: number;
    latency_ms: number;
    node_name: string;
}

export interface NodeStatus {
    node_id: string;
    node_name: string;
    tier: string;
    protocol_version: string;
    capabilities: string[];
    tokens_per_sec: number;
    memory_usage_pct: number;
    queue_depth: number;
    uptime_secs: number;
}

export class LaRucheClient {
    private baseUrl: string;

    constructor(baseUrl: string) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
    }

    private request<T>(path: string, method: string = 'GET', body?: object): Promise<T> {
        return new Promise((resolve, reject) => {
            const url = new URL(this.baseUrl + path);
            const isHttps = url.protocol === 'https:';
            const lib = isHttps ? https : http;

            const options: http.RequestOptions = {
                hostname: url.hostname,
                port: url.port,
                path: url.pathname,
                method,
                headers: { 'Content-Type': 'application/json' },
                timeout: 600000, // 10 minutes for slow local models
            };

            const req = lib.request(options, (res) => {
                let data = '';
                res.on('data', chunk => data += chunk);
                res.on('end', () => {
                    if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
                        try {
                            resolve(JSON.parse(data) as T);
                        } catch {
                            reject(new Error(`Invalid JSON: ${data}`));
                        }
                    } else {
                        reject(new Error(`HTTP ${res.statusCode}: ${data}`));
                    }
                });
            });

            req.on('error', reject);
            req.on('timeout', () => {
                req.destroy();
                reject(new Error('Request timeout'));
            });

            if (body) {
                req.write(JSON.stringify(body));
            }
            req.end();
        });
    }

    async status(): Promise<NodeStatus> {
        return this.request<NodeStatus>('/');
    }

    async swarm(): Promise<SwarmData> {
        return this.request<SwarmData>('/swarm');
    }

    async infer(prompt: string, capability: string = 'llm'): Promise<InferResponse> {
        return this.request<InferResponse>('/infer', 'POST', {
            prompt,
            capability,
            qos: 'normal',
        });
    }

    async health(): Promise<boolean> {
        try {
            const resp = await this.request<string>('/health');
            return true;
        } catch {
            return false;
        }
    }
}
