#!/usr/bin/env python3
"""Probe actual host Vulkan ICD initialization with an extracted Argand bundle."""
import argparse
import ctypes as c
import json
import os
from pathlib import Path
import subprocess
import sys


def probe():
    vulkan = c.CDLL('libvulkan.so.1')
    class Application(c.Structure):
        _fields_ = [('type', c.c_uint32), ('next', c.c_void_p), ('name', c.c_char_p),
                    ('version', c.c_uint32), ('engine', c.c_char_p),
                    ('engine_version', c.c_uint32), ('api', c.c_uint32)]
    class Instance(c.Structure):
        _fields_ = [('type', c.c_uint32), ('next', c.c_void_p), ('flags', c.c_uint32),
                    ('application', c.POINTER(Application)), ('layer_count', c.c_uint32),
                    ('layers', c.c_void_p), ('extension_count', c.c_uint32),
                    ('extensions', c.c_void_p)]
    class Extension(c.Structure):
        _fields_ = [('name', c.c_char * 256), ('version', c.c_uint32)]
    count = c.c_uint32()
    vulkan.vkEnumerateInstanceExtensionProperties.argtypes = [c.c_char_p, c.POINTER(c.c_uint32), c.c_void_p]
    if vulkan.vkEnumerateInstanceExtensionProperties(None, c.byref(count), None):
        raise ValueError('cannot enumerate Vulkan instance extensions')
    extensions = (Extension * count.value)()
    if vulkan.vkEnumerateInstanceExtensionProperties(None, c.byref(count), extensions):
        raise ValueError('cannot read Vulkan instance extensions')
    names = {entry.name.decode() for entry in extensions}
    surfaces = sorted(names & {'VK_KHR_xcb_surface', 'VK_KHR_xlib_surface', 'VK_KHR_wayland_surface'})
    if 'VK_KHR_surface' not in names or not surfaces:
        raise ValueError('Vulkan ICDs expose no usable desktop surface extensions')
    application = Application(type=0, name=b'Argand package probe', api=(1 << 22))
    info = Instance(type=1, application=c.pointer(application))
    instance = c.c_void_p()
    vulkan.vkCreateInstance.argtypes = [c.POINTER(Instance), c.c_void_p, c.POINTER(c.c_void_p)]
    vulkan.vkEnumeratePhysicalDevices.argtypes = [c.c_void_p, c.POINTER(c.c_uint32), c.c_void_p]
    vulkan.vkDestroyInstance.argtypes = [c.c_void_p, c.c_void_p]
    result = vulkan.vkCreateInstance(c.byref(info), None, c.byref(instance))
    if result:
        raise ValueError(f'vkCreateInstance failed: {result}')
    try:
        count = c.c_uint32()
        result = vulkan.vkEnumeratePhysicalDevices(instance, c.byref(count), None)
        if result or not count.value:
            raise ValueError(f'Vulkan ICD enumeration failed: result={result}, devices={count.value}')
        print(json.dumps({'vulkan_instance': 'success', 'physical_devices': count.value, 'surface_extensions': surfaces,
                          'library_path': os.environ.get('LD_LIBRARY_PATH', '')}))
    finally:
        vulkan.vkDestroyInstance(instance, None)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bundle', type=Path)
    parser.add_argument('--worker', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.worker:
        probe()
        return
    bundle = args.bundle.resolve()
    if not (bundle / 'argand-bin').is_file() or not (bundle / 'lib').is_dir():
        raise ValueError('expected an extracted Linux Argand bundle')
    environment = dict(os.environ)
    environment['LD_LIBRARY_PATH'] = str(bundle / 'lib') + (':' + environment['LD_LIBRARY_PATH'] if environment.get('LD_LIBRARY_PATH') else '')
    subprocess.run([sys.executable, __file__, str(bundle), '--worker'], env=environment, check=True)


if __name__ == '__main__':
    main()
