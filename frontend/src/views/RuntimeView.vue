<template>
  <LayoutContent
    :title="t('runtime.title')"
    :description="t('pageDesc.runtimes')"
    reload
    @reload="invalidate"
  >
    <template #toolbar>
      <FpSelectButton
        v-model="activeKind"
        :options="kindOptions"
        option-label="label"
        option-value="value"
        :allow-empty="false"
      />
      <FpSwitch
        v-if="activeKind === 'node'"
        v-model="withPm2"
        class="runtime-pm2"
        :label="t('runtime.node.withPm2')"
      />
    </template>

    <p v-if="hintKinds.includes(activeKind)" class="runtime-hint">
      {{ t(`runtime.${activeKind}.hint`) }}
    </p>

    <FpStatePanel
      :loading="availableQuery.loading.value"
      :error="availableQuery.error.value"
      :empty="availableList.length === 0"
      retryable
      @retry="availableQuery.refresh"
    >
      <div class="runtime-grid">
        <FpCard
          v-for="item in availableList"
          :key="item.version"
          :title="`v${item.version}`"
          class="runtime-card"
        >
          <template #extra>
            <FpTag v-if="item.is_default" :value="t('runtime.default')" severity="info" />
            <FpTag v-else-if="item.installed" :value="t('runtime.installed')" severity="success" />
            <FpTag v-else :value="t('runtime.notInstalled')" severity="neutral" />
          </template>

          <div class="runtime-card__body">
            <div class="runtime-card__meta">
              <span>{{ t('runtime.kindLabel') }}：{{ kindLabel(activeKind) }}</span>
              <span v-if="item.installed && item.runtime_id">
                {{ t('runtime.installPath') }}：{{ installPathOf(item.runtime_id) }}
              </span>
              <span v-if="item.runtime_id && metaOf(item.runtime_id).version">
                {{ t('runtime.resolvedVersion') }}：v{{ metaOf(item.runtime_id).version }}
                <template v-if="metaOf(item.runtime_id).vendor">
                  · {{ metaOf(item.runtime_id).vendor }}
                </template>
                <FpTag
                  v-if="metaOf(item.runtime_id).pm2"
                  value="pm2"
                  severity="info"
                  class="runtime-pm2-tag"
                />
              </span>
            </div>
            <div class="runtime-card__actions">
              <FpButton
                v-permission="{ perm: 'runtime:create', mode: 'view' }"
                variant="primary"
                size="small"
                :loading="installingVersion === item.version"
                :disabled="item.installed"
                @click="handleInstall(item)"
              >
                {{ t('common.install') }}
              </FpButton>
              <FpButton
                v-if="item.installed && !item.is_default && item.runtime_id"
                v-permission="{ perm: 'runtime:update', mode: 'view' }"
                variant="secondary"
                size="small"
                :loading="defaultingId === item.runtime_id"
                @click="handleSetDefault(item.runtime_id)"
              >
                {{ t('runtime.setDefault') }}
              </FpButton>
              <FpButton
                v-if="item.installed && item.runtime_id"
                v-permission="{ perm: 'runtime:delete', mode: 'view' }"
                variant="danger"
                size="small"
                @click="handleUninstall(item)"
              >
                {{ t('runtime.uninstall') }}
              </FpButton>
            </div>
          </div>
        </FpCard>
      </div>
    </FpStatePanel>
  </LayoutContent>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import LayoutContent from '@/components/ui/LayoutContent.vue'
import FpCard from '@/components/ui/FpCard.vue'
import FpTag from '@/components/ui/FpTag.vue'
import FpButton from '@/components/ui/FpButton.vue'
import FpSelectButton from '@/components/ui/FpSelectButton.vue'
import FpSwitch from '@/components/ui/FpSwitch.vue'
import FpStatePanel from '@/components/ui/FpStatePanel.vue'
import { useFpToast } from '@/components/ui/FpToast'
import { useFpConfirm } from '@/components/ui/FpConfirm'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import { RUNTIME_KINDS } from '@/api/runtimes'
import type { RuntimeKind } from '@/api/runtimes'
import {
  availableRuntimes,
  installRuntime,
  uninstallRuntime,
  setDefaultRuntime,
  listRuntimes,
} from '@/api/runtimes'
import type { RuntimeVersionInfo, RuntimeDto } from '@/api/generated'

const { t } = useI18n()
const toast = useFpToast()
const { confirmAction } = useFpConfirm()

const activeKind = ref<RuntimeKind>('php')
const installingVersion = ref('')
const defaultingId = ref<number | null>(null)
/** S-11：Node 安装时同时 `npm install -g pm2` */
const withPm2 = ref(false)

const kindOptions = computed(() => RUNTIME_KINDS.map((k) => ({ label: kindLabel(k), value: k })))

function kindLabel(kind: RuntimeKind): string {
  return t(`runtime.kinds.${kind}`)
}

/** 已登记运行时（用于补全安装路径等扩展信息） */
const runtimesQuery = useApiQuery<RuntimeDto[]>(
  () => queryKeys.runtimes.list(),
  async () => {
    const res = await listRuntimes()
    return { data: res.data }
  },
)
const installedMap = computed(() => {
  const map = new Map<number, RuntimeDto>()
  for (const r of runtimesQuery.data.value ?? []) map.set(r.id, r)
  return map
})

function installPathOf(runtimeId: number): string {
  return installedMap.value.get(runtimeId)?.install_path ?? ''
}

/** 有安装说明的语言（目录式安装：版本解析 / 镜像 / 校验 / 项目命令约定） */
const hintKinds: readonly RuntimeKind[] = ['node', 'python', 'go', 'java']

interface InstallerMeta {
  version?: string
  pm2?: boolean
  vendor?: string
}

/** 安装器自述（解析后的真实版本 / Node 是否带 pm2 / JDK 发行商），元数据损坏时为空对象 */
function metaOf(runtimeId: number): InstallerMeta {
  const raw = installedMap.value.get(runtimeId)?.metadata_json
  if (!raw) return {}
  try {
    const v = JSON.parse(raw) as unknown
    return v && typeof v === 'object' ? (v as InstallerMeta) : {}
  } catch {
    return {}
  }
}

const availableQuery = useApiQuery<RuntimeVersionInfo[]>(
  () => queryKeys.runtimes.available(activeKind.value),
  async () => {
    const res = await availableRuntimes(activeKind.value)
    return { data: res.data }
  },
  { keepPrevious: true },
)
const availableList = computed<RuntimeVersionInfo[]>(() => availableQuery.data.value ?? [])

function invalidate() {
  availableQuery.refresh()
  runtimesQuery.refresh()
}

async function handleInstall(item: RuntimeVersionInfo) {
  installingVersion.value = item.version
  try {
    await installRuntime({
      kind: activeKind.value,
      version: item.version,
      ...(activeKind.value === 'node' && withPm2.value ? { pm2: true } : {}),
    })
    toast.success(t('runtime.installSuccess'))
    invalidate()
  } catch (err) {
    toast.error(err, t('runtime.installFailed'))
  } finally {
    installingVersion.value = ''
  }
}

async function handleSetDefault(id: number) {
  defaultingId.value = id
  try {
    await setDefaultRuntime(id)
    toast.success(t('runtime.defaultSuccess'))
    invalidate()
  } catch (err) {
    toast.error(err, t('runtime.defaultFailed'))
  } finally {
    defaultingId.value = null
  }
}

function handleUninstall(item: RuntimeVersionInfo) {
  if (!item.runtime_id) return
  confirmAction({
    message: t('runtime.uninstallConfirm', { version: item.version }),
    header: t('runtime.uninstall'),
    accept: async () => {
      try {
        await uninstallRuntime(item.runtime_id as number)
        toast.success(t('runtime.uninstallSuccess'))
        invalidate()
      } catch (err) {
        toast.error(err, t('runtime.uninstallFailed'))
      }
    },
  })
}
</script>

<style scoped>
.runtime-pm2 {
  margin-left: var(--fp-space-3);
}
.runtime-hint {
  margin: 0 0 var(--fp-space-3);
  font-size: 13px;
  color: var(--fp-text-secondary);
}
.runtime-pm2-tag {
  margin-left: var(--fp-space-1);
}
.runtime-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: var(--fp-space-4);
}
.runtime-card__body {
  display: flex;
  flex-direction: column;
  gap: var(--fp-space-3);
}
.runtime-card__meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
  color: var(--fp-text-secondary);
  word-break: break-all;
}
.runtime-card__actions {
  display: flex;
  gap: var(--fp-space-2);
  flex-wrap: wrap;
}
</style>
