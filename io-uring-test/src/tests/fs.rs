use crate::utils;
use crate::Test;
use io_uring::{opcode, types, IoUring};
use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};

pub fn test_file_write_read(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require!(
        test;
        test.probe.is_supported(opcode::Write::CODE);
        test.probe.is_supported(opcode::Read::CODE);
    );

    println!("test file_write_read");

    let fd = tempfile::tempfile()?;
    let fd = types::Fd(fd.as_raw_fd());

    utils::write_read(ring, fd, fd)?;

    Ok(())
}

pub fn test_file_writev_readv(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require!(
        test;
        test.probe.is_supported(opcode::Writev::CODE);
        test.probe.is_supported(opcode::Readv::CODE);
    );

    println!("test file_writev_readv");

    let fd = tempfile::tempfile()?;
    let fd = types::Fd(fd.as_raw_fd());

    utils::writev_readv(ring, fd, fd)?;

    Ok(())
}

pub fn test_file_fsync(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require!(
        test;
        test.probe.is_supported(opcode::Fsync::CODE);
    );

    println!("test file_fsync");

    let mut fd = tempfile::tempfile()?;
    let n = fd.write(&[0x1])?;
    assert_eq!(n, 1);

    let fd = types::Fd(fd.as_raw_fd());

    let fsync_e = opcode::Fsync::new(fd);

    unsafe {
        ring.submission()
            .push(&fsync_e.build().user_data(0x03))
            .expect("queue is full");
    }

    ring.submit_and_wait(1)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), 1);
    assert_eq!(cqes[0].user_data(), 0x03);
    assert_eq!(cqes[0].result(), 0);

    Ok(())
}

pub fn test_file_fsync_file_range(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require!(
        test;
        test.probe.is_supported(opcode::SyncFileRange::CODE);
    );

    println!("test file_fsync_file_range");

    let mut fd = tempfile::tempfile()?;
    let n = fd.write(&[0x2; 3 * 1024])?;
    assert_eq!(n, 3 * 1024);
    let n = fd.write(&[0x3; 1024])?;
    assert_eq!(n, 1024);

    let fd = types::Fd(fd.as_raw_fd());

    let fsync_e = opcode::SyncFileRange::new(fd, 1024).offset(3 * 1024);

    unsafe {
        ring.submission()
            .push(&fsync_e.build().user_data(0x04))
            .expect("queue is full");
    }

    ring.submit_and_wait(1)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), 1);
    assert_eq!(cqes[0].user_data(), 0x04);
    assert_eq!(cqes[0].result(), 0);

    Ok(())
}

pub fn test_file_fallocate(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require!(
        test;
        test.probe.is_supported(opcode::Fallocate::CODE);
    );

    println!("test file_fallocate");

    let fd = tempfile::tempfile()?;
    let fd = types::Fd(fd.as_raw_fd());

    let falloc_e = opcode::Fallocate::new(fd, 1024);

    unsafe {
        ring.submission()
            .push(&falloc_e.build().user_data(0x10))
            .expect("queue is full");
    }

    ring.submit_and_wait(1)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), 1);
    assert_eq!(cqes[0].user_data(), 0x10);
    assert_eq!(cqes[0].result(), 0);

    Ok(())
}

pub fn test_file_openat2(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require!(
        test;
        test.probe.is_supported(opcode::OpenAt2::CODE);
    );

    use tempfile::tempdir;

    println!("test file_openat2");

    let dir = tempdir()?;
    let dirfd = types::Fd(libc::AT_FDCWD);

    let path = dir.path().join("test-io-uring-openat2");
    let path = CString::new(path.as_os_str().as_bytes())?;

    let openhow = types::OpenHow::new().flags(libc::O_CREAT as _);
    let open_e = opcode::OpenAt2::new(dirfd, path.as_ptr(), &openhow);

    unsafe {
        ring.submission()
            .push(&open_e.build().user_data(0x11))
            .expect("queue is full");
    }

    ring.submit_and_wait(1)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), 1);
    assert_eq!(cqes[0].user_data(), 0x11);
    assert!(cqes[0].result() > 0);

    let fd = unsafe { fs::File::from_raw_fd(cqes[0].result()) };

    assert!(fd.metadata()?.is_file());

    Ok(())
}

pub fn test_file_close(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require!(
        test;
        test.probe.is_supported(opcode::Close::CODE);
    );

    println!("test file_cloes");

    let fd = tempfile::tempfile()?;
    let fd = types::Fd(fd.into_raw_fd());

    let close_e = opcode::Close::new(fd);

    unsafe {
        ring.submission()
            .push(&close_e.build().user_data(0x12))
            .expect("queue is full");
    }

    ring.submit_and_wait(1)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), 1);
    assert_eq!(cqes[0].user_data(), 0x12);
    assert_eq!(cqes[0].result(), 0);

    Ok(())
}

pub fn test_file_cur_pos(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require!(
        test;
        test.probe.is_supported(opcode::Write::CODE);
        test.probe.is_supported(opcode::Read::CODE);
        ring.params().is_feature_rw_cur_pos();
    );

    println!("test file_cur_pos");

    let fd = tempfile::tempfile()?;
    let fd = types::Fd(fd.into_raw_fd());

    let text = b"The quick brown fox jumps over the lazy dog.";
    let mut output = vec![0; text.len()];

    let write_e = opcode::Write::new(fd, text.as_ptr(), 22)
        .offset(-1)
        .build()
        .user_data(0x01);

    unsafe {
        ring.submission().push(&write_e).expect("queue is full");
    }

    ring.submit_and_wait(1)?;

    let write_e = opcode::Write::new(fd, unsafe { text.as_ptr().add(22) }, 22)
        .offset(-1)
        .build()
        .user_data(0x02);

    unsafe {
        ring.submission().push(&write_e).expect("queue is full");
    }

    ring.submit_and_wait(2)?;

    let read_e = opcode::Read::new(fd, output.as_mut_ptr(), output.len() as _);

    unsafe {
        ring.submission()
            .push(&read_e.build().user_data(0x03))
            .expect("queue is full");
    }

    ring.submit_and_wait(3)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), 3);
    assert_eq!(cqes[0].user_data(), 0x01);
    assert_eq!(cqes[1].user_data(), 0x02);
    assert_eq!(cqes[2].user_data(), 0x03);
    assert_eq!(cqes[0].result(), 22);
    assert_eq!(cqes[1].result(), 22);
    assert_eq!(cqes[2].result(), text.len() as i32);

    assert_eq!(&output, text);

    Ok(())
}

#[cfg(not(feature = "ci"))]
pub fn test_statx(ring: &mut IoUring, test: &Test) -> anyhow::Result<()> {
    require!(
        test;
        test.probe.is_supported(opcode::Statx::CODE);
    );

    println!("test statx");

    let dir = tempfile::tempdir()?;
    let path = dir.path().join("test-io-uring-statx");
    let pathbuf = CString::new(path.as_os_str().as_bytes())?;
    fs::write(&path, "1")?;

    let mut statxbuf: libc::statx = unsafe { std::mem::zeroed() };

    let statx_e = opcode::Statx::new(
        types::Fd(libc::AT_FDCWD),
        pathbuf.as_ptr(),
        &mut statxbuf as *mut libc::statx as *mut _,
    )
    .mask(libc::STATX_ALL)
    .build()
    .user_data(0x99);

    unsafe {
        ring.submission().push(&statx_e).expect("queue is full");
    }

    ring.submit_and_wait(1)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), 1);
    assert_eq!(cqes[0].user_data(), 0x99);
    assert_eq!(cqes[0].result(), 0);

    // check
    let mut statxbuf2 = unsafe { std::mem::zeroed() };
    let ret = unsafe {
        libc::statx(
            libc::AT_FDCWD,
            pathbuf.as_ptr(),
            0,
            libc::STATX_ALL,
            &mut statxbuf2,
        )
    };

    assert_eq!(ret, 0);
    assert_eq!(statxbuf, statxbuf2);

    // statx fd
    let fd = fs::File::open(&path)?;
    let mut statxbuf3: libc::statx = unsafe { std::mem::zeroed() };

    let statx_e = opcode::Statx::new(
        types::Fd(fd.as_raw_fd()),
        b"\0".as_ptr().cast(),
        &mut statxbuf3 as *mut libc::statx as *mut _,
    )
    .flags(libc::AT_EMPTY_PATH)
    .mask(libc::STATX_ALL)
    .build()
    .user_data(0x9a);

    unsafe {
        ring.submission().push(&statx_e).expect("queue is full");
    }

    ring.submit_and_wait(1)?;

    let cqes = ring.completion().collect::<Vec<_>>();

    assert_eq!(cqes.len(), 1);
    assert_eq!(cqes[0].user_data(), 0x9a);
    assert_eq!(cqes[0].result(), 0);

    assert_eq!(statxbuf3, statxbuf2);

    Ok(())
}
