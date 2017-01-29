use ffi;
use libc::{c_int, c_long, c_ulong};
use std::ptr;
use std::mem;

use {cvt, cvt_p};
use asn1::Asn1GeneralizedTimeRef;
use error::ErrorStack;
use hash::MessageDigest;
use stack::StackRef;
use types::OpenSslTypeRef;
use x509::store::X509StoreRef;
use x509::{X509, X509Ref};

bitflags! {
    pub flags Flag: c_ulong {
        const FLAG_NO_CERTS = ffi::OCSP_NOCERTS,
        const FLAG_NO_INTERN = ffi::OCSP_NOINTERN,
        const FLAG_NO_CHAIN = ffi::OCSP_NOCHAIN,
        const FLAG_NO_VERIFY = ffi::OCSP_NOVERIFY,
        const FLAG_NO_EXPLICIT = ffi::OCSP_NOEXPLICIT,
        const FLAG_NO_CA_SIGN = ffi::OCSP_NOCASIGN,
        const FLAG_NO_DELEGATED = ffi::OCSP_NODELEGATED,
        const FLAG_NO_CHECKS = ffi::OCSP_NOCHECKS,
        const FLAG_TRUST_OTHER = ffi::OCSP_TRUSTOTHER,
        const FLAG_RESPID_KEY = ffi::OCSP_RESPID_KEY,
        const FLAG_NO_TIME = ffi::OCSP_NOTIME,
    }
}

pub const RESPONSE_STATUS_SUCCESSFUL: OcspResponseStatus =
    OcspResponseStatus(ffi::OCSP_RESPONSE_STATUS_SUCCESSFUL);
pub const RESPONSE_STATUS_MALFORMED_REQUEST: OcspResponseStatus =
    OcspResponseStatus(ffi::OCSP_RESPONSE_STATUS_MALFORMEDREQUEST);
pub const RESPONSE_STATUS_INTERNAL_ERROR: OcspResponseStatus =
    OcspResponseStatus(ffi::OCSP_RESPONSE_STATUS_INTERNALERROR);
pub const RESPONSE_STATUS_TRY_LATER: OcspResponseStatus =
    OcspResponseStatus(ffi::OCSP_RESPONSE_STATUS_TRYLATER);
pub const RESPONSE_STATUS_SIG_REQUIRED: OcspResponseStatus =
    OcspResponseStatus(ffi::OCSP_RESPONSE_STATUS_SIGREQUIRED);
pub const RESPONSE_STATUS_UNAUTHORIZED: OcspResponseStatus =
    OcspResponseStatus(ffi::OCSP_RESPONSE_STATUS_UNAUTHORIZED);

pub const CERT_STATUS_GOOD: OcspCertStatus = OcspCertStatus(ffi::V_OCSP_CERTSTATUS_GOOD);
pub const CERT_STATUS_REVOKED: OcspCertStatus = OcspCertStatus(ffi::V_OCSP_CERTSTATUS_REVOKED);
pub const CERT_STATUS_UNKNOWN: OcspCertStatus = OcspCertStatus(ffi::V_OCSP_CERTSTATUS_UNKNOWN);

pub const REVOKED_STATUS_NO_STATUS: OcspRevokedStatus =
    OcspRevokedStatus(ffi::OCSP_REVOKED_STATUS_NOSTATUS);
pub const REVOKED_STATUS_UNSPECIFIED: OcspRevokedStatus =
    OcspRevokedStatus(ffi::OCSP_REVOKED_STATUS_UNSPECIFIED);
pub const REVOKED_STATUS_KEY_COMPROMISE: OcspRevokedStatus =
    OcspRevokedStatus(ffi::OCSP_REVOKED_STATUS_KEYCOMPROMISE);
pub const REVOKED_STATUS_CA_COMPROMISE: OcspRevokedStatus =
    OcspRevokedStatus(ffi::OCSP_REVOKED_STATUS_CACOMPROMISE);
pub const REVOKED_STATUS_AFFILIATION_CHANGED: OcspRevokedStatus =
    OcspRevokedStatus(ffi::OCSP_REVOKED_STATUS_AFFILIATIONCHANGED);
pub const REVOKED_STATUS_SUPERSEDED: OcspRevokedStatus =
    OcspRevokedStatus(ffi::OCSP_REVOKED_STATUS_SUPERSEDED);
pub const REVOKED_STATUS_CESSATION_OF_OPERATION: OcspRevokedStatus =
    OcspRevokedStatus(ffi::OCSP_REVOKED_STATUS_CESSATIONOFOPERATION);
pub const REVOKED_STATUS_CERTIFICATE_HOLD: OcspRevokedStatus =
    OcspRevokedStatus(ffi::OCSP_REVOKED_STATUS_CERTIFICATEHOLD);
pub const REVOKED_STATUS_REMOVE_FROM_CRL: OcspRevokedStatus =
    OcspRevokedStatus(ffi::OCSP_REVOKED_STATUS_REMOVEFROMCRL);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OcspResponseStatus(c_int);

impl OcspResponseStatus {
    pub fn from_raw(raw: c_int) -> OcspResponseStatus {
        OcspResponseStatus(raw)
    }

    pub fn as_raw(&self) -> c_int {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OcspCertStatus(c_int);

impl OcspCertStatus {
    pub fn from_raw(raw: c_int) -> OcspCertStatus {
        OcspCertStatus(raw)
    }

    pub fn as_raw(&self) -> c_int {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OcspRevokedStatus(c_int);

impl OcspRevokedStatus {
    pub fn from_raw(raw: c_int) -> OcspRevokedStatus {
        OcspRevokedStatus(raw)
    }

    pub fn as_raw(&self) -> c_int {
        self.0
    }
}

pub struct Status<'a> {
    /// The overall status of the response.
    pub status: OcspCertStatus,
    /// If `status` is `CERT_STATUS_REVOKED`, the reason for the revocation.
    pub reason: OcspRevokedStatus,
    /// If `status` is `CERT_STATUS_REVOKED`, the time at which the certificate was revoked.
    pub revocation_time: Option<&'a Asn1GeneralizedTimeRef>,
    /// The time that this revocation check was performed.
    pub this_update: &'a Asn1GeneralizedTimeRef,
    /// The time at which this revocation check expires.
    pub next_update: &'a Asn1GeneralizedTimeRef,
}

impl<'a> Status<'a> {
    /// Checks validity of the `this_update` and `next_update` fields.
    ///
    /// The `nsec` parameter specifies an amount of slack time that will be used when comparing
    /// those times with the current time to account for delays and clock skew.
    ///
    /// The `maxsec` parameter limits the maximum age of the `this_update` parameter to prohibit
    /// very old responses.
    pub fn check_validity(&self, nsec: u32, maxsec: Option<u32>) -> Result<(), ErrorStack> {
        unsafe {
            cvt(ffi::OCSP_check_validity(self.this_update.as_ptr(),
                                         self.next_update.as_ptr(),
                                         nsec as c_long,
                                         maxsec.map(|n| n as c_long).unwrap_or(-1)))
                .map(|_| ())
        }
    }
}

type_!(OcspBasicResponse, OcspBasicResponseRef, ffi::OCSP_BASICRESP, ffi::OCSP_BASICRESP_free);

impl OcspBasicResponseRef {
    /// Verifies the validity of the response.
    ///
    /// The `certs` parameter contains a set of certificates that will be searched when locating the
    /// OCSP response signing certificate. Some responders do not include this in the response.
    pub fn verify(&self,
                  certs: &StackRef<X509>,
                  store: &X509StoreRef,
                  flags: Flag)
                  -> Result<(), ErrorStack> {
        unsafe {
            cvt(ffi::OCSP_basic_verify(self.as_ptr(), certs.as_ptr(), store.as_ptr(), flags.bits()))
                .map(|_| ())
        }
    }

    /// Looks up the status for the specified certificate ID.
    pub fn find_status<'a>(&'a self, id: &OcspCertIdRef) -> Option<Status<'a>> {
        unsafe {
            let mut status = ffi::V_OCSP_CERTSTATUS_UNKNOWN;
            let mut reason = ffi::OCSP_REVOKED_STATUS_NOSTATUS;
            let mut revocation_time = ptr::null_mut();
            let mut this_update = ptr::null_mut();
            let mut next_update = ptr::null_mut();

            let r = ffi::OCSP_resp_find_status(self.as_ptr(),
                                               id.as_ptr(),
                                               &mut status,
                                               &mut reason,
                                               &mut revocation_time,
                                               &mut this_update,
                                               &mut next_update);
            if r == 1 {
                let revocation_time = if revocation_time.is_null() {
                    None
                } else {
                    Some(Asn1GeneralizedTimeRef::from_ptr(revocation_time))
                };
                Some(Status {
                    status: OcspCertStatus(status),
                    reason: OcspRevokedStatus(status),
                    revocation_time: revocation_time,
                    this_update: Asn1GeneralizedTimeRef::from_ptr(this_update),
                    next_update: Asn1GeneralizedTimeRef::from_ptr(next_update),
                })
            } else {
                None
            }
        }
    }
}

type_!(OcspCertId, OcspCertIdRef, ffi::OCSP_CERTID, ffi::OCSP_CERTID_free);

impl OcspCertId {
    /// Constructs a certificate ID for certificate `subject`.
    pub fn from_cert(digest: MessageDigest,
                     subject: &X509Ref,
                     issuer: &X509Ref)
                     -> Result<OcspCertId, ErrorStack> {
        unsafe {
            cvt_p(ffi::OCSP_cert_to_id(digest.as_ptr(), subject.as_ptr(), issuer.as_ptr()))
                .map(OcspCertId)
        }
    }
}

type_!(OcspResponse, OcspResponseRef, ffi::OCSP_RESPONSE, ffi::OCSP_RESPONSE_free);

impl OcspResponse {
    /// Creates an OCSP response from the status and optional body.
    ///
    /// A body should only be provided if `status` is `RESPONSE_STATUS_SUCCESSFUL`.
    pub fn create(status: OcspResponseStatus,
                  body: Option<&OcspBasicResponseRef>)
                  -> Result<OcspResponse, ErrorStack> {
        unsafe {
            ffi::init();

            cvt_p(ffi::OCSP_response_create(status.as_raw(),
                                            body.map(|r| r.as_ptr()).unwrap_or(ptr::null_mut())))
                .map(OcspResponse)
        }
    }

    from_der!(OcspResponse, ffi::d2i_OCSP_RESPONSE);
}

impl OcspResponseRef {
    to_der!(ffi::i2d_OCSP_RESPONSE);

    /// Returns the status of the response.
    pub fn status(&self) -> OcspResponseStatus {
        unsafe {
            OcspResponseStatus(ffi::OCSP_response_status(self.as_ptr()))
        }
    }

    /// Returns the basic response.
    ///
    /// This will only succeed if `status()` returns `RESPONSE_STATUS_SUCCESSFUL`.
    pub fn basic(&self) -> Result<OcspBasicResponse, ErrorStack> {
        unsafe {
            cvt_p(ffi::OCSP_response_get1_basic(self.as_ptr())).map(OcspBasicResponse)
        }
    }
}

type_!(OcspRequest, OcspRequestRef, ffi::OCSP_REQUEST, ffi::OCSP_REQUEST_free);

impl OcspRequest {
    pub fn new() -> Result<OcspRequest, ErrorStack> {
        unsafe {
            ffi::init();

            cvt_p(ffi::OCSP_REQUEST_new()).map(OcspRequest)
        }
    }

    from_der!(OcspRequest, ffi::d2i_OCSP_REQUEST);
}

impl OcspRequestRef {
    to_der!(ffi::i2d_OCSP_REQUEST);

    pub fn add_id(&mut self, id: OcspCertId) -> Result<&mut OcspOneReqRef, ErrorStack> {
        unsafe {
            let ptr = try!(cvt_p(ffi::OCSP_request_add0_id(self.as_ptr(), id.as_ptr())));
            mem::forget(id);
            Ok(OcspOneReqRef::from_ptr_mut(ptr))
        }
    }
}

type_!(OcspOneReq, OcspOneReqRef, ffi::OCSP_ONEREQ, ffi::OCSP_ONEREQ_free);