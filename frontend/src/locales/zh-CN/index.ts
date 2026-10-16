import common from './common'
import shell from './shell'
import auth from './auth'
import dashboard from './dashboard'
import memo from './memo'
import users from './users'
import nodes from './nodes'
import files from './files'
import databases from './databases'
import websites from './websites'
import docker from './docker'
import plugins from './plugins'
import security from './security'
import logs from './logs'
import tasks from './tasks'
import settings from './settings'
import backup from './backup'
import runtime from './runtime'
import webServers from './webServers'
import appStore from './appStore'
import projects from './projects'
import terminal from './terminal'
import inspection from './inspection'
import inspectionFindings from './inspectionFindings'
import processes from './processes'
import toolbox from './toolbox'
import apiDocs from './apiDocs'
import aiCenter from './aiCenter'
import assistant from './assistant'
import twoFactor from './twoFactor'

export default {
  ...common,
  ...shell,
  ...auth,
  ...dashboard,
  ...memo,
  ...users,
  ...nodes,
  ...files,
  ...databases,
  ...websites,
  ...docker,
  ...plugins,
  ...security,
  ...logs,
  ...tasks,
  ...settings,
  ...backup,
  ...runtime,
  ...webServers,
  ...appStore,
  ...projects,
  ...terminal,
  ...inspection,
  ...inspectionFindings,
  ...processes,
  ...toolbox,
  ...apiDocs,
  ...aiCenter,
  ...assistant,
  ...twoFactor,
}
