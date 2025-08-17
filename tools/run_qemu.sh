#!/usr/bin/env bash
set -euo pipefail

EFI_PATH="${1:?usage: qemu-run.sh <path-to-efi>}"

RUN_DIR="$(dirname "$0")"
ESP_DIR="${RUN_DIR}/esp" 
IMG_PATH="${RUN_DIR}/esp.img" 

qemu-img create -f raw "${IMG_PATH}" 64M

mkfs.fat -F 32 "${IMG_PATH}"

mkdir -p "${ESP_DIR}/EFI/BOOT"
cp -f "${EFI_PATH}" "${ESP_DIR}/EFI/BOOT/BOOTX64.EFI"

mkdir -p "${ESP_DIR}/kernel"
cp -f "${RUN_DIR}/kernel.elf" "${ESP_DIR}/kernel/kernel.elf"
cp -f "${RUN_DIR}/init.elf" "${ESP_DIR}/kernel/init.elf"

mcopy -i "${IMG_PATH}" -s "${ESP_DIR}"/* ::/

rm -rf "${ESP_DIR}"

exec qemu-system-x86_64 \
  -m 512 -cpu max -net none -serial stdio \
  -drive if=pflash,format=raw,readonly=on,file="${RUN_DIR}/OVMF_CODE.fd" \
  -drive if=pflash,format=raw,file="${RUN_DIR}/OVMF_VARS.fd" \
  -drive format=raw,file="${IMG_PATH}"

