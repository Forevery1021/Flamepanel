/**
 * DNS 厂商账号 API（主线三 §6.1 Stage 4 · /api/dns-providers）。
 * 类型来自 OpenAPI 生成（src/api/generated），禁止手写重复定义。
 * 凭据 credentials_json 仅在创建/更新请求中出现，响应一律不回显。
 */
import api from './client'
import type { DnsProviderAccountDto } from '@/api/generated'

export interface CreateDnsProviderRequest {
  name: string
  /** cloudflare | aliyun | tencent | huawei */
  provider_type: string
  /** 厂商凭据 JSON 字符串（落库前服务端统一加密） */
  credentials_json?: string
  enabled?: boolean
}

export interface UpdateDnsProviderRequest {
  name?: string
  provider_type?: string
  credentials_json?: string
  enabled?: boolean
}

export function listDnsProviders() {
  return api.get<DnsProviderAccountDto[]>('/dns-providers')
}

export function createDnsProvider(input: CreateDnsProviderRequest) {
  return api.post<DnsProviderAccountDto>('/dns-providers', input)
}

export function updateDnsProvider(id: number, input: UpdateDnsProviderRequest) {
  return api.put<DnsProviderAccountDto>(`/dns-providers/${id}`, input)
}

export function deleteDnsProvider(id: number) {
  return api.delete(`/dns-providers/${id}`)
}

/** 连通性测试（Stage 4 凭据结构校验），失败返回 200 + last_test_status=failed */
export function testDnsProvider(id: number) {
  return api.post<DnsProviderAccountDto>(`/dns-providers/${id}/test`)
}
