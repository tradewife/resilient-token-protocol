// RTP CLI — Railway GraphQL API client.

import fs from "fs";

const RAILWAY_GRAPHQL_URL = "https://backboard.railway.com/graphql/v2";
const RTP_ENV_ID = "986bee12-1028-4016-aa42-ba0a174233b4";

// Known service IDs
export const SERVICE_IDS: Record<string, string> = {
  "rtp-dashboard": "f44e64aa-81d0-429d-b3e5-605d72ef2778",
  // Others: check via railway service list
};

export interface RailwayService {
  name: string;
  id: string;
  status: string;
  lastDeployAt: string | null;
  cronSchedule: string | null;
  url: string | null;
}

export function loadRailwayToken(tokenPath: string | null): string | null {
  if (!tokenPath) return null;
  try {
    if (!fs.existsSync(tokenPath)) return null;
    return fs.readFileSync(tokenPath, "utf-8").trim();
  } catch {
    return null;
  }
}

async function railwayQuery(token: string, query: string, variables?: Record<string, unknown>): Promise<unknown> {
  const resp = await fetch(RAILWAY_GRAPHQL_URL, {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query, variables }),
  });
  if (!resp.ok) throw new Error(`Railway API error: ${resp.status} ${resp.statusText}`);
  return resp.json();
}

export async function fetchServiceStatus(token: string): Promise<RailwayService[]> {
  // Query all services in the project
  const query = `
    query($projectId: String!, $environmentId: String!) {
      project(id: $projectId) {
        services {
          edges {
            node {
              id
              name
              serviceInstances {
                edges {
                  node {
                    id
                    environmentId
                    latestDeployment {
                      status
                      createdAt
                    }
                    source {
                      cronSchedule
                    }
                    domains {
                      edges {
                        node {
                          domain
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  `;
  // Use the environment ID to filter instances
  const result = await railwayQuery(token, query, {
    projectId: "11004852-2ba7-46d9-aeb5-ab9558e965a0",
    environmentId: RTP_ENV_ID,
  }) as any;

  const services: RailwayService[] = [];
  try {
    const edges = result?.data?.project?.services?.edges ?? [];
    for (const edge of edges) {
      const node = edge.node;
      const instances = node.serviceInstances?.edges ?? [];
      const instance = instances.find(
        (i: any) => i.node.environmentId === RTP_ENV_ID,
      );
      if (!instance) continue;
      const inst = instance.node;
      const latestDeploy = inst.latestDeployment;
      const domains = inst.domains?.edges ?? [];
      const domain = domains[0]?.node?.domain ?? null;

      services.push({
        name: node.name,
        id: node.id,
        status: latestDeploy?.status ?? "unknown",
        lastDeployAt: latestDeploy?.createdAt ?? null,
        cronSchedule: inst.source?.cronSchedule ?? null,
        url: domain ? `https://${domain}` : null,
      });
    }
  } catch {
    // Return empty if parsing fails
  }
  return services;
}
