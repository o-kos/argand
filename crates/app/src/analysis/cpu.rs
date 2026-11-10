//! Best-effort core-class discovery; CPU identifiers always come from the OS.

use std::io;

pub fn efficiency_cpus() -> io::Result<Option<Vec<usize>>> {
    std::thread::Builder::new()
        .name("argand-topology".to_owned())
        .spawn(platform::discover)?
        .join()
        .map_err(|_| io::Error::other("CPU topology probe panicked"))?
}

pub fn pin_current(cpus: &[usize]) -> io::Result<()> {
    platform::pin(cpus)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod platform {
    use super::io;
    use std::arch::x86_64::__cpuid;

    fn mask() -> libc::cpu_set_t {
        // cpu_set_t is an integer bitset; zero denotes the empty set.
        unsafe { std::mem::zeroed() }
    }

    fn allowed() -> io::Result<Vec<usize>> {
        let mut set = mask();
        // The kernel writes only the supplied cpu_set_t-sized buffer.
        if unsafe { libc::sched_getaffinity(0, std::mem::size_of_val(&set), &mut set) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok((0..libc::CPU_SETSIZE as usize)
            .filter(|&cpu| unsafe { libc::CPU_ISSET(cpu, &set) })
            .collect())
    }

    pub fn pin(cpus: &[usize]) -> io::Result<()> {
        if cpus.is_empty() || cpus.iter().any(|&cpu| cpu >= libc::CPU_SETSIZE as usize) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid CPU set",
            ));
        }
        let mut set = mask();
        for &cpu in cpus {
            // Every index was checked against CPU_SETSIZE above.
            unsafe {
                libc::CPU_SET(cpu, &mut set);
            }
        }
        // A zero pid changes only the calling thread's affinity.
        if unsafe { libc::sched_setaffinity(0, std::mem::size_of_val(&set), &set) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub fn discover() -> io::Result<Option<Vec<usize>>> {
        // CPUID is available on x86_64; vendor/leaf guards precede hybrid queries.
        let base = __cpuid(0);
        let vendor = [
            base.ebx.to_le_bytes(),
            base.edx.to_le_bytes(),
            base.ecx.to_le_bytes(),
        ]
        .concat();
        if base.eax < 0x1a || vendor != b"GenuineIntel" {
            return Ok(None);
        }
        let mut efficient = Vec::new();
        for cpu in allowed()? {
            pin(&[cpu])?;
            // Intel CPUID.1A EAX[31:24]: 0x20 identifies Atom-class hybrid cores.
            if __cpuid(0x1a).eax >> 24 == 0x20 {
                efficient.push(cpu);
            }
        }
        // The probing thread exits; its temporary masks never affect its parent.
        Ok((!efficient.is_empty()).then_some(efficient))
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod platform {
    use super::io;
    pub fn discover() -> io::Result<Option<Vec<usize>>> {
        Ok(None)
    }
    pub fn pin(_: &[usize]) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "CPU affinity is unsupported on this platform",
        ))
    }
}
