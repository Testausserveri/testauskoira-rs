extern crate photon_rs;

use photon_rs::native::*;

pub async fn build_award_image(user_img_url: &str) -> Result<String, ()> {
    let profile_picture = reqwest::get(format!("{}?size=128", user_img_url))
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();

    //FIXME open_image_from_bytes causes monochrome image, webp bad, fuck.
    let pfp_photon = open_image_from_bytes(profile_picture.as_ref()).expect("The profile-pic should be open");
    let mask_photon = open_image("img/blackcomposite.png").expect("mask.png should be open");
    let mut pfp_photon = photon_rs::transform::resize(&pfp_photon,mask_photon.get_width(),mask_photon.get_height(),photon_rs::transform::SamplingFilter::Gaussian);

    photon_rs::multiple::watermark(&mut pfp_photon, &mask_photon, 0, 0);
    save_image(pfp_photon, "pfp_new.png");

    Ok("pfp_new.png".to_string())
}
