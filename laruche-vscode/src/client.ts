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
    port?: number | null;
    capabilities: string[];
    /** Primary model running on this node (from LAND TXT broadcast) */
    model: string | null;
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
    cpu_usage_pct: number;
    memory_used_mb: number;
    memory_total_mb: number;
    queue_depth: number;
    uptime_secs: number;
}

export interface OllamaModel {
    name: string;
    size_gb: number;
    digest: string;
}

export interface ModelsResponse {
    models: OllamaModel[];
    default_model: string;
}

export class LaRucheClient {
    private baseUrl: string;

    constructor(baseUrl: string) {
        this.baseUrl = baseUrl.replace(/\/$/, '');
    }

    getBaseUrl(): string {
        return this.baseUrl;
    }

    setBaseUrl(url: string): void {
        this.baseUrl = url.replace(/\/$/, '');
    }

    private request<T>(
        path: string,
        method: string = 'GET',
        body?: object,
        timeoutMs: number = 10000,
    ): Promise<T> {
        return new Promise((resolve, reject) => {
            const url = new URL(this.baseUrl + path);
            const isHttps = url.protocol === 'https:';
            const lib = isHttps ? https : http;

            const options: http.RequestOptions = {
                hostname: url.hostname,
                port: url.port,
                path: `${url.pathname}${url.search}`,
                method,
                headers: { 'Content-Type': 'application/json' },
                timeout: timeoutMs,
            };

            const req = lib.request(options, (res) => {
                let data = '';
                res.on('data', chunk => { data += chunk; });
                res.on('end', () => {
                    if (res.statusCode && res.statusCode >= 200 && res.statusCode < 300) {
                        const payload = data.trim();
                        if (!payload) {
                            resolve(undefined as T);
                            return;
                        }
                        try {
                            resolve(JSON.parse(payload) as T);
                        } catch {
                            resolve(payload as unknown as T);
                        }
                    } else {
                        reject(new Error(`HTTP ${res.statusCode}: ${data.slice(0, 200)}`));
                    }
                });
            });

            req.on('error', reject);
            req.on('timeout', () => {
                req.destroy();
                reject(new Error(`Request timeout after ${timeoutMs}ms`));
            });

            if (body) {
                req.write(JSON.stringify(body));
            }
            req.end();
        });
    }

    async status(): Promise<NodeStatus> {
        return this.request<NodeStatus>('/', 'GET', undefined, 5000);
    }

    async swarm(): Promise<SwarmData> {
        return this.request<SwarmData>('/swarm', 'GET', undefined, 5000);
    }

    async models(): Promise<ModelsResponse> {
        return this.request<ModelsResponse>('/models', 'GET', undefined, 5000);
    }

    /**
     * Run inference on the node.
     * @param prompt The prompt to send.
     * @param capability Capability type ('llm', 'code', 'vlm', ...).
     * @param model Optional model override. Uses node default if not specified.
     */
    async infer(
        prompt: string,
        capability: string = 'llm',
        model?: string,
    ): Promise<InferResponse> {
        const body: Record<string, unknown> = {
            prompt,
            capability,
            qos: 'normal',
        };
        if (model) {
            body['model'] = model;
        }
        // Long timeout for slow local models (up to 10 minutes)
        return this.request<InferResponse>('/infer', 'POST', body, 600000);
    }

    /**
     * Run inference with a shorter timeout suitable for interactive chat.
     * Defaults to 120s instead of 10min.
     */
    async inferChat(
        prompt: string,
        capability: string = 'llm',
        model?: string,
        timeoutMs: number = 120000,
    ): Promise<InferResponse> {
        const body: Record<string, unknown> = {
            prompt,
            capability,
            qos: 'normal',
        };
        if (model) {
            body['model'] = model;
        }
        return this.request<InferResponse>('/infer', 'POST', body, timeoutMs);
    }

    async health(timeoutMs: number = 3000): Promise<boolean> {
        try {
            const response = await this.request<unknown>('/health', 'GET', undefined, timeoutMs);
            if (typeof response === 'string') {
                return response.trim().toLowerCase() === 'ok';
            }
            return true;
        } catch {
            return false;
        }
    }
}

