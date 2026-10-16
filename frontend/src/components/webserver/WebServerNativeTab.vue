<template>
  <div class="panel">
    <div class="native-toolbar">
      <FpButton variant="primary" icon="oi oi-refresh" :loading="detecting" @click="fetchNative">
        {{ t('webServer.nativeDetect') }}
      </FpButton>
      <FpInlineMessage severity="info" class="native-hint">
        {{ t('webServer.nativeHint') }}
      </FpInlineMessage>
    </div>
    <FpTable
      :rows="nativeList"
      :loading="detecting"
      :empty-text="t('common.noData')"
      striped-rows
      class="mt-2"
    >
      <FpColumn field="engine" :header="t('webServer.engine')" style="width: 130px" />
      <FpColumn :header="t('webServer.status')" style="width: 110px">
        <template #body="{ data }">
          <FpTag :severity="data.installed ? 'success' : 'info'">
            {{
              data.installed ? t('webServer.nativeInstalled') : t('webServer.nativeNotInstalled')
            }}
          </FpTag>
        </template>
      </FpColumn>
      <FpColumn :header="t('webServer.nativeVersion')" style="width: 100px">
        <template #body="{ data }">{{ data.version || '—' }}</template>
      </FpColumn>
      <FpColumn :header="t('webServer.nativeRunning')" style="width: 90px">
        <template #body="{ data }">
          <FpTag :severity="data.running ? 'success' : 'info'">
            {{ data.running ? t('webServer.nativeRunning') : t('webServer.nativeStopped') }}
          </FpTag>
        </template>
      </FpColumn>
      <FpColumn :header="t('webServer.nativeAutostart')" style="width: 110px">
        <template #body="{ data }">
          <FpSwitch
            :model-value="data.enabled"
            :disabled="!data.installed"
            @update:model-value="(v) => handleNativeAutostart(data, Boolean(v))"
          />
        </template>
      </FpColumn>
      <FpColumn :header="t('webServer.nativeListenPorts')" style="width: 140px">
        <template #body="{ data }">
          <template v-if="data.listening_ports && data.listening_ports.length">
            <FpTag v-for="p in data.listening_ports" :key="p" class="mr-1">{{ p }}</FpTag>
          </template>
          <span v-else>—</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('webServer.nativePath')" style="min-width: 160px">
        <template #body="{ data }">
          <span v-if="data.binary_path" v-tooltip="data.binary_path" class="cell-truncate">{{
            data.binary_path
          }}</span>
          <span v-else>—</span>
        </template>
      </FpColumn>
      <FpColumn :header="t('webServer.actions')" style="width: 280px" frozen>
        <template #body="{ data }">
          <div class="row-actions">
            <FpButton
              v-if="!data.installed"
              variant="primary"
              @click="confirmNativeInstall(data)"
              >{{ t('webServer.nativeInstallBtn') }}</FpButton
            >
            <FpButton v-else variant="danger" @click="confirmNativeUninstall(data)">{{
              t('webServer.nativeUninstallBtn')
            }}</FpButton>
            <FpButton variant="ghost" @click="openCapabilities(data)">{{
              t('webServer.capabilities')
            }}</FpButton>
          </div>
        </template>
      </FpColumn>
    </FpTable>
  </div>

  <FpModal v-model="showCapabilities" :header="capabilitiesTitle" size="lg">
    <div class="modal-form">
      <FpInlineMessage severity="info" class="preset-alert">
        {{ t('webServer.capabilitiesHint') }}
      </FpInlineMessage>
      <div v-if="capabilitiesLoading" class="cap-loading">{{ t('common.loading') }}</div>
      <div v-else class="cap-list">
        <div v-for="cap in capabilityList" :key="cap.key" class="cap-item">
          <FpCheckbox v-model="selectedCapabilities[cap.key]" class="cap-check">
            {{ cap.label_zh }}
          </FpCheckbox>
          <div class="cap-detail">
            <span class="cap-packages">
              {{
                cap.packages && cap.packages.length
                  ? cap.packages.join(', ')
                  : t('webServer.capBuiltin')
              }}
            </span>
            <span class="cap-hint">{{ cap.hint }}</span>
          </div>
        </div>
      </div>
    </div>
    <template #footer>
      <FpButton variant="ghost" @click="showCapabilities = false">{{
        t('common.cancel')
      }}</FpButton>
      <FpButton variant="primary" :loading="capInstalling" @click="handleInstallCapabilities">
        {{ t('common.install') }}
      </FpButton>
    </template>
  </FpModal>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  detectNativeWebServers,
  nativeInstallWebServer,
  nativeUninstallWebServer,
  nativeAutostartWebServer,
  getEngineCapabilities,
  installCapabilities,
} from '@/api/webServers'
import type { NativeWebServerInfo, CapabilityInfo } from '@/types'
import FpTable from '@/components/ui/FpTable.vue'
import FpColumn from '@/components/ui/FpColumn.vue'
import FpModal from '@/components/ui/FpModal.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpCheckbox from '@/components/ui/FpCheckbox.vue'
import FpInlineMessage from '@/components/ui/FpInlineMessage.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'

/**
 * Web 服务器 ·「本机原生」页签（从 WebServersView 拆出）：探测系统包管理器安装的引擎，
 * 安装 / 卸载 / 开机自启，以及按需安装配套能力（模块 / 扩展包）。
 * 安装或卸载成功后 emit `changed`，由父级刷新「面板实例」列表。
 */
const emit = defineEmits<{ changed: [] }>()
const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

// ── 原生控制 ──
const nativeList = ref<NativeWebServerInfo[]>([])
const detecting = ref(false)
const nativeBusy = ref(false)

async function fetchNative() {
  detecting.value = true
  try {
    const res = await detectNativeWebServers()
    nativeList.value = res.data
  } catch {
    toast.error(t('common.failed'))
  } finally {
    detecting.value = false
  }
}

function confirmNativeInstall(row: NativeWebServerInfo) {
  confirmAction({
    message: t('webServer.nativeInstallConfirm', { name: row.engine }),
    header: t('common.confirmAction'),
    accept: async () => {
      if (nativeBusy.value) return
      nativeBusy.value = true
      try {
        await nativeInstallWebServer(row.engine)
        toast.success(t('common.success'))
        await fetchNative()
        emit('changed')
      } catch (e: unknown) {
        toast.error(e, t('common.failed'))
      } finally {
        nativeBusy.value = false
      }
    },
  })
}

function confirmNativeUninstall(row: NativeWebServerInfo) {
  confirmAction({
    message: t('webServer.nativeUninstallConfirm', { name: row.engine }),
    header: t('common.confirmAction'),
    accept: async () => {
      if (nativeBusy.value) return
      nativeBusy.value = true
      try {
        await nativeUninstallWebServer(row.engine)
        toast.success(t('common.success'))
        await fetchNative()
        emit('changed')
      } catch (e: unknown) {
        toast.error(e, t('common.failed'))
      } finally {
        nativeBusy.value = false
      }
    },
  })
}

async function handleNativeAutostart(row: NativeWebServerInfo, enabled: boolean) {
  try {
    await nativeAutostartWebServer(row.engine, enabled)
    toast.success(t('common.success'))
    await fetchNative()
  } catch {
    toast.error(t('common.failed'))
  }
}

// ── 配套能力（安装时"各取所需"） ──
const showCapabilities = ref(false)
const capabilitiesLoading = ref(false)
const capInstalling = ref(false)
const capabilityList = ref<CapabilityInfo[]>([])
const selectedCapabilities = ref<Record<string, boolean>>({})
const capabilitiesTitle = ref('')
let capabilitiesEngine = ''

async function openCapabilities(row: NativeWebServerInfo) {
  capabilitiesEngine = row.engine
  capabilitiesTitle.value = `${t('webServer.capabilities')} - ${row.engine}`
  capabilityList.value = []
  selectedCapabilities.value = {}
  showCapabilities.value = true
  capabilitiesLoading.value = true
  try {
    const res = await getEngineCapabilities(row.engine)
    capabilityList.value = res.data.capabilities ?? []
  } catch {
    toast.error(t('common.failed'))
  } finally {
    capabilitiesLoading.value = false
  }
}

async function handleInstallCapabilities() {
  const picked = Object.keys(selectedCapabilities.value).filter(
    (k) => selectedCapabilities.value[k],
  )
  if (!capabilitiesEngine || !picked.length) {
    toast.warning(t('common.required'))
    return
  }
  capInstalling.value = true
  try {
    const res = await installCapabilities(capabilitiesEngine, picked)
    const installed = res.data.installed?.length ?? 0
    const failed = res.data.failed?.length ?? 0
    if (res.data.success) {
      toast.success(t('webServer.capInstalled', { count: String(installed) }))
    } else {
      toast.warning(t('webServer.capPartial', { ok: String(installed), fail: String(failed) }))
    }
    showCapabilities.value = false
    selectedCapabilities.value = {}
  } catch {
    toast.error(t('common.failed'))
  } finally {
    capInstalling.value = false
  }
}

onMounted(() => {
  fetchNative()
})
</script>

<style scoped>
.native-toolbar {
  display: flex;
  align-items: center;
  gap: var(--fp-space-3);
  flex-wrap: wrap;
  margin-bottom: var(--fp-space-4);
}
.native-hint {
  flex: 1;
  min-width: 260px;
}
.cap-loading {
  padding: var(--fp-space-4);
  text-align: center;
  color: var(--fp-text-muted);
}
.cap-list {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.cap-item {
  display: flex;
  align-items: flex-start;
  gap: var(--fp-space-2);
  padding: var(--fp-space-3);
  border: 1px solid var(--fp-border);
  border-radius: var(--fp-radius-md);
}
.cap-check {
  flex-shrink: 0;
}
.cap-detail {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.cap-packages {
  font-size: 0.85rem;
  color: var(--fp-text-primary);
  font-weight: 500;
}
.cap-hint {
  font-size: 0.8rem;
  color: var(--fp-text-muted);
}
.row-actions {
  display: flex;
  gap: var(--fp-space-1);
  flex-wrap: wrap;
}
.cell-truncate {
  display: inline-block;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.mr-1 {
  margin-right: 4px;
}
.preset-alert {
  width: 100%;
}
</style>
