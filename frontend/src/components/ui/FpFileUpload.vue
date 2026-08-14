<template>
  <FileUpload v-bind="$attrs" class="fp-fileupload" />
</template>

<script setup lang="ts">
import FileUpload from 'openvue/fileupload'
</script>

<style scoped>
.fp-fileupload {
  width: 100%;
}
</style>
