import type { Ref } from 'vue'
import { watch } from 'vue'
import {
  listContainers,
  listImages,
  listNetworks,
  listVolumes,
  listComposeProjects,
} from '@/api/docker'
import { useApiQuery } from '@/composables/useApiQuery'
import { queryKeys } from '@/api/queryKeys'
import type {
  DockerContainer,
  DockerImage,
  DockerNetwork,
  DockerVolume,
  ComposeProject,
} from '@/types'

export type DockerResourceKind = 'containers' | 'images' | 'networks' | 'volumes' | 'compose'

export interface DockerResource<T> {
  /** 当前资源数据 */
  data: Ref<T[] | undefined>
  /** 是否加载中 */
  loading: Ref<boolean>
  /** 请求错误文案（无错误为 null） */
  error: Ref<string | null>
  /** 手动刷新（写操作后调用；也用于 enabled 变 true 时首次加载） */
  refresh: () => void
}

type ResourceData<K extends DockerResourceKind> = K extends 'containers'
  ? DockerContainer
  : K extends 'images'
    ? DockerImage
    : K extends 'networks'
      ? DockerNetwork
      : K extends 'volumes'
        ? DockerVolume
        : ComposeProject

/**
 * Y2 · useDockerResource — 统一 Docker 各资源（容器/镜像/网络/卷/Compose）的数据获取层。
 *
 * 将 DockerView 中分散的手写 `loading/ref/try-catch` 收敛为 useApiQuery：
 * - 享受统一缓存、去重、staleTime 与错误规范化
 * - 通过 `enabled`（可为 Ref）配合 tab 惰性加载：首次切换 tab 时 enabled 置 true，
 *   内部 watch 自动触发首次加载，避免首屏 5 倍请求
 * - 写操作完成后调用 `refresh()` 即可重新拉取
 */
export function useDockerResource<K extends DockerResourceKind>(
  resource: K,
  enabled: Ref<boolean> | boolean = true,
): DockerResource<ResourceData<K>> {
  const enabledRef = typeof enabled === 'boolean' ? { value: enabled } : enabled

  const q = useDockerQuery(resource, enabledRef.value)
  watch(enabledRef, (v) => {
    if (v) void q.refresh()
  })
  return q as DockerResource<ResourceData<K>>
}

function useDockerQuery(resource: DockerResourceKind, enabled: boolean) {
  switch (resource) {
    case 'containers': {
      const q = useApiQuery<DockerContainer[]>(
        queryKeys.containers.list(),
        () => listContainers(),
        { enabled },
      )
      return { data: q.data, loading: q.loading, error: q.error, refresh: q.refresh }
    }
    case 'images': {
      const q = useApiQuery<DockerImage[]>(queryKeys.images.list(), () => listImages(), {
        enabled,
      })
      return { data: q.data, loading: q.loading, error: q.error, refresh: q.refresh }
    }
    case 'networks': {
      const q = useApiQuery<DockerNetwork[]>(queryKeys.networks.list(), () => listNetworks(), {
        enabled,
      })
      return { data: q.data, loading: q.loading, error: q.error, refresh: q.refresh }
    }
    case 'volumes': {
      const q = useApiQuery<DockerVolume[]>(queryKeys.volumes.list(), () => listVolumes(), {
        enabled,
      })
      return { data: q.data, loading: q.loading, error: q.error, refresh: q.refresh }
    }
    case 'compose': {
      const q = useApiQuery<ComposeProject[]>(
        queryKeys.composeProjects.list(),
        () => listComposeProjects(),
        { enabled },
      )
      return { data: q.data, loading: q.loading, error: q.error, refresh: q.refresh }
    }
  }
}
