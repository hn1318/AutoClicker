<script setup lang="ts">
interface Props {
  title: string;
  description?: string;
}

defineProps<Props>();
</script>

<template>
  <section class="section">
    <header class="section__head">
      <h2 class="section__title">{{ title }}</h2>
      <div class="section__action">
        <slot name="action" />
      </div>
    </header>
    <p v-if="description" class="section__desc">{{ description }}</p>
    <div class="section__body panel">
      <slot />
    </div>
  </section>
</template>

<style scoped>
.section {
  display: flex;
  flex-direction: column;
}

.section__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--sp-4);
  min-height: 18px;
  margin-right: 2px;
  margin-bottom: var(--sp-4);
}

.section__title {
  font-size: var(--fs-11);
  font-weight: var(--fw-medium);
  line-height: 1;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-muted);
}

.section__desc {
  margin: calc(var(--sp-2) * -1) 0 var(--sp-4);
  font-size: var(--fs-12);
  color: var(--text-faint);
}

.section__action {
  display: flex;
  align-items: center;
  gap: var(--sp-2);
}
</style>
