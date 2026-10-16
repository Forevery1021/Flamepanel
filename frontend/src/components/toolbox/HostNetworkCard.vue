<template>
  <section class="panel">
    <header class="panel-header">
      <span class="panel-title">{{ t('toolbox.network.title') }}</span>
      <FpSwitch
        v-if="virtualCount > 0"
        v-model="showVirtual"
        :label="t('toolbox.network.showVirtual', { n: virtualCount })"
      />
    </header>

    <div class="table-wrap">
      <table class="kv-table">
        <thead>
          <tr>
            <th>{{ t('toolbox.network.name') }}</th>
            <th>{{ t('toolbox.network.addresses') }}</th>
            <th>MAC</th>
            <th class="num">{{ t('toolbox.network.rx') }}</th>
            <th class="num">{{ t('toolbox.network.tx') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="iface in rows" :key="iface.name">
            <td class="mono">
              {{ iface.name }}
              <FpTag
                v-if="iface.virtual_iface"
                severity="neutral"
                :value="t('toolbox.network.virtual')"
              />
            </td>
            <td class="mono addr">
              <span v-for="a in iface.addresses" :key="a">{{ a }}</span>
              <span v-if="!iface.addresses.length" class="text-muted">—</span>
            </td>
            <td class="mono">{{ iface.mac ?? '—' }}</td>
            <td class="mono num">{{ formatSize(iface.rx_bytes) }}</td>
            <td class="mono num">{{ formatSize(iface.tx_bytes) }}</td>
          </tr>
          <tr v-if="!rows.length">
            <td colspan="5" class="text-muted empty">{{ t('toolbox.network.empty') }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="sub-grid">
      <div>
        <h4 class="sub-title">{{ t('toolbox.dns.title') }}</h4>
        <dl class="dns-list">
          <dt>{{ t('toolbox.dns.nameservers') }}</dt>
          <dd class="mono">{{ joinOrDash(dns.nameservers) }}</dd>
          <dt>{{ t('toolbox.dns.search') }}</dt>
          <dd class="mono">{{ joinOrDash(dns.search) }}</dd>
          <dt>{{ t('toolbox.dns.options') }}</dt>
          <dd class="mono">{{ joinOrDash(dns.options) }}</dd>
        </dl>
      </div>
      <div>
        <h4 class="sub-title">
          {{ t('toolbox.hosts.title') }}
          <span class="text-muted">({{ hosts.length }}{{ hostsTruncated ? '+' : '' }})</span>
        </h4>
        <ul v-if="hosts.length" class="hosts-list">
          <li v-for="(h, idx) in hosts" :key="`${idx}:${h.ip}`" class="mono">
            <span class="hosts-ip">{{ h.ip }}</span>
            <span>{{ h.names.join(' ') }}</span>
          </li>
        </ul>
        <p v-else class="text-muted">{{ t('toolbox.hosts.empty') }}</p>
        <p v-if="hostsTruncated" class="text-muted">{{ t('toolbox.hosts.truncated') }}</p>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpTag from '@/components/ui/FpTag.vue'
import type { DnsConfig, HostInterface, HostsEntry } from '@/api/toolbox'
import { formatSize } from '@/utils/format'
import { visibleInterfaces } from '@/utils/hostInfo'

const props = defineProps<{
  interfaces: HostInterface[]
  dns: DnsConfig
  hosts: HostsEntry[]
  hostsTruncated: boolean
}>()

const { t } = useI18n()

const showVirtual = ref(false)
const virtualCount = computed(() => props.interfaces.filter((i) => i.virtual_iface).length)
const rows = computed(() => visibleInterfaces(props.interfaces, showVirtual.value))

function joinOrDash(list: string[]): string {
  return list.length ? list.join(', ') : '—'
}
</script>

<style scoped>
.table-wrap {
  overflow-x: auto;
}
.kv-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}
.kv-table th {
  text-align: left;
  font-weight: 500;
  font-size: var(--fp-font-xs);
  color: var(--fp-text-muted);
  padding: var(--fp-space-2) var(--fp-space-3);
  border-bottom: 1px solid var(--fp-border);
}
.kv-table td {
  padding: var(--fp-space-2) var(--fp-space-3);
  border-bottom: 1px solid var(--fp-border);
  vertical-align: top;
}
.num {
  text-align: right;
  white-space: nowrap;
}
.addr {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.empty {
  text-align: center;
}
.sub-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--fp-space-4);
  margin-top: var(--fp-space-4);
}
.sub-title {
  margin: 0 0 var(--fp-space-2);
  font-size: var(--fp-font-sm);
  font-weight: 600;
  color: var(--fp-text-primary);
}
.dns-list {
  display: grid;
  grid-template-columns: max-content minmax(0, 1fr);
  gap: var(--fp-space-1) var(--fp-space-3);
  margin: 0;
  font-size: 13px;
}
.dns-list dt {
  color: var(--fp-text-muted);
}
.dns-list dd {
  margin: 0;
  word-break: break-all;
}
.hosts-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 220px;
  overflow-y: auto;
  font-size: 13px;
}
.hosts-list li {
  display: flex;
  gap: var(--fp-space-3);
  padding: 2px 0;
}
.hosts-ip {
  min-width: 9rem;
  color: var(--fp-text-secondary);
}
@media (max-width: 720px) {
  .sub-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
